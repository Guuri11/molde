//! Cliente LSP mínimo que habla con molde-lsp en memoria, con el mismo protocolo que un editor.

use molde_core::Engine;
use serde_json::{Value, json};
use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream, ReadHalf, WriteHalf};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tower_lsp_server::ls_types::Uri;

pub struct Lsp {
    writer: WriteHalf<DuplexStream>,
    rx: UnboundedReceiver<Value>,
    next: i64,
    /// Notificaciones y peticiones que ha enviado el servidor.
    pub received: Vec<Value>,
    pub utf8: bool,
    pub capabilities: Value,
}

pub fn uri(path: &Path) -> String {
    Uri::from_file_path(path)
        .expect("ruta absoluta")
        .as_str()
        .to_string()
}

pub fn path_of(uri: &str) -> std::path::PathBuf {
    let u: Uri = uri.parse().expect("uri");
    u.to_file_path().expect("uri de fichero").into_owned()
}

impl Lsp {
    pub async fn start(engine: Engine, root: &Path, utf8: bool) -> Lsp {
        let (service, socket) = molde_lsp::service(engine);
        let (client_io, server_io) = tokio::io::duplex(1 << 22);
        let (sr, sw) = tokio::io::split(server_io);
        tokio::spawn(tower_lsp_server::Server::new(sr, sw, socket).serve(service));
        let (cr, cw) = tokio::io::split(client_io);
        let (tx, rx) = unbounded_channel();
        tokio::spawn(read_loop(cr, tx));
        let mut lsp = Lsp {
            writer: cw,
            rx,
            next: 1,
            received: Vec::new(),
            utf8,
            capabilities: Value::Null,
        };
        let mut caps = json!({ "window": { "workDoneProgress": true } });
        if utf8 {
            caps["general"] = json!({ "positionEncodings": ["utf-8", "utf-16"] });
        }
        let root_uri = uri(root);
        let res = lsp
            .request(
                "initialize",
                json!({ "processId": null, "rootUri": root_uri, "capabilities": caps, "workspaceFolders": [{ "uri": root_uri, "name": "proyecto" }] }),
            )
            .await;
        lsp.capabilities = res["result"]["capabilities"].clone();
        lsp.notify("initialized", json!({})).await;
        lsp
    }

    async fn send(&mut self, msg: Value) {
        let body = msg.to_string();
        let frame = format!("Content-Length: {}\r\n\r\n{body}", body.len());
        self.writer
            .write_all(frame.as_bytes())
            .await
            .expect("escribir al servidor");
    }

    /// Envía una petición y devuelve la respuesta completa. Contesta por el camino a las peticiones del servidor.
    pub async fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.send_request(method, params).await;
        self.response(id).await
    }

    /// Envía una petición sin esperar la respuesta. Devuelve su id.
    pub async fn send_request(&mut self, method: &str, params: Value) -> i64 {
        let id = self.next;
        self.next += 1;
        self.send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))
            .await;
        id
    }

    /// La respuesta a `id`, aunque haya llegado mientras se esperaba otra.
    pub async fn response(&mut self, id: i64) -> Value {
        let is_response = |m: &Value| m.get("method").is_none() && m["id"] == id;
        if let Some(i) = self.received.iter().position(is_response) {
            return self.received.remove(i);
        }
        loop {
            let msg = tokio::time::timeout(Duration::from_secs(60), self.rx.recv())
                .await
                .unwrap_or_else(|_| panic!("sin respuesta a la petición {id}"))
                .expect("el servidor ha cerrado");
            if is_response(&msg) {
                return msg;
            }
            self.take(msg).await;
        }
    }

    /// Guarda un mensaje del servidor y, si es una petición, la contesta como un editor.
    async fn take(&mut self, msg: Value) {
        if msg.get("method").is_some() && msg.get("id").is_some() {
            let result = if msg["method"] == "window/showDocument" {
                json!({ "success": true })
            } else {
                Value::Null
            };
            self.send(json!({ "jsonrpc": "2.0", "id": msg["id"], "result": result }))
                .await;
        }
        self.received.push(msg);
    }

    pub async fn notify(&mut self, method: &str, params: Value) {
        self.send(json!({ "jsonrpc": "2.0", "method": method, "params": params }))
            .await;
    }

    /// Si ya ha llegado la respuesta a `id`, sin esperar.
    pub async fn answered(&mut self, id: i64) -> bool {
        while let Ok(m) = self.rx.try_recv() {
            self.take(m).await;
        }
        self.received
            .iter()
            .any(|m| m.get("method").is_none() && m["id"] == id)
    }

    pub async fn exec(&mut self, command: &str, arguments: Value) -> Value {
        self.request(
            "workspace/executeCommand",
            json!({ "command": command, "arguments": arguments }),
        )
        .await
    }

    /// Mensajes recibidos de un método, con sus params. Las notificaciones del servidor pueden
    /// llegar después de la respuesta que las provocó: una ida y vuelta más las trae todas.
    pub async fn messages(&mut self, method: &str) -> Vec<Value> {
        self.request("molde/ping", Value::Null).await;
        while let Ok(m) = self.rx.try_recv() {
            self.received.push(m);
        }
        self.received
            .iter()
            .filter(|m| m["method"] == method)
            .map(|m| m["params"].clone())
            .collect()
    }
}

async fn read_loop(mut reader: ReadHalf<DuplexStream>, tx: UnboundedSender<Value>) {
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = vec![0u8; 1 << 16];
    loop {
        while let Some(end) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            let header = String::from_utf8_lossy(&buf[..end]).to_string();
            let len: usize = header
                .lines()
                .find_map(|l| l.strip_prefix("Content-Length: "))
                .and_then(|n| n.trim().parse().ok())
                .expect("cabecera Content-Length");
            if buf.len() < end + 4 + len {
                break;
            }
            let body: Vec<u8> = buf.drain(..end + 4 + len).skip(end + 4).collect();
            if tx
                .send(serde_json::from_slice(&body).expect("JSON del servidor"))
                .is_err()
            {
                return;
            }
        }
        match reader.read(&mut chunk).await {
            Ok(0) | Err(_) => return,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    }
}

/// Posición LSP de un byte, contada por el cliente en su codificación.
pub fn pos(text: &str, byte: usize, utf8: bool) -> Value {
    let before = &text[..byte];
    let start = before.rfind('\n').map_or(0, |i| i + 1);
    let col = &before[start..];
    let character = if utf8 {
        col.len()
    } else {
        col.encode_utf16().count()
    };
    json!({ "line": before.matches('\n').count(), "character": character })
}

fn offset(text: &str, p: &Value, utf8: bool) -> usize {
    let (line, character) = (
        p["line"].as_u64().unwrap() as usize,
        p["character"].as_u64().unwrap() as usize,
    );
    let start: usize = text.split_inclusive('\n').take(line).map(str::len).sum();
    let mut units = 0;
    for (i, ch) in text[start..].char_indices() {
        if units == character {
            return start + i;
        }
        assert!(
            units < character && ch != '\n',
            "posición fuera de la línea o en mitad de un carácter: {p}"
        );
        units += if utf8 { ch.len_utf8() } else { ch.len_utf16() };
    }
    text.len()
}

/// Aplica la edición principal y las adicionales de un item, como haría el editor.
pub fn apply(text: &str, item: &Value, utf8: bool) -> String {
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    let mut push = |e: &Value| {
        let (s, t) = (
            offset(text, &e["range"]["start"], utf8),
            offset(text, &e["range"]["end"], utf8),
        );
        edits.push((s, t, e["newText"].as_str().unwrap_or_default().to_string()));
    };
    // El editor expande el snippet: aquí, con los valores por defecto.
    let mut main = item["textEdit"].clone();
    let body = main["newText"].as_str().unwrap_or_default().to_string();
    main["newText"] =
        Value::from(molde_core::body::expand(&body).expect("cuerpo de snippet válido"));
    push(&main);
    for e in item["additionalTextEdits"].as_array().into_iter().flatten() {
        push(e);
    }
    edits.sort_by_key(|e| std::cmp::Reverse(e.0));
    let mut out = text.to_string();
    for (s, t, new) in edits {
        out.replace_range(s..t, &new);
    }
    out
}
