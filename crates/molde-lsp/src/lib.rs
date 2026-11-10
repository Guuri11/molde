//! Adaptador LSP.

use molde_core::{
    Assistant, CompletionRequest, Engine, Layer, Notice, ScaffoldError, SnippetId,
    candidates_markdown, stats_markdown,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use tower_lsp_server::jsonrpc::Result;
use tower_lsp_server::ls_types::*;
use tower_lsp_server::{
    Client, ClientSocket, LanguageServer, LspService, NotCancellable, OngoingProgress, Server,
    Unbounded,
};

/// Comandos que el servidor declara al iniciar. Zed solo ejecuta los declarados.
pub const COMMANDS: [&str; 5] = [
    "molde.recordUse",
    "molde.stats",
    "molde.candidates",
    "molde.scaffold",
    "molde.capture",
];

pub const CAPTURE_TITLE: &str = "Guardar selección como snippet";

pub struct Backend {
    client: Client,
    engine: Mutex<Engine>,
    docs: Mutex<HashMap<Uri, Doc>>,
    state: Mutex<State>,
}

struct Doc {
    text: String,
    language: String,
}

#[derive(Default)]
struct State {
    utf8: bool,
    root: Option<PathBuf>,
    /// El cliente admite `window/workDoneProgress/create`.
    progress: bool,
}

/// Tokens de progreso únicos en la sesión.
static NEXT_PROGRESS: AtomicI32 = AtomicI32::new(1);

type CommandResult = std::result::Result<(Option<(PathBuf, u32)>, Vec<Notice>), String>;

pub fn service(engine: Engine) -> (LspService<Backend>, ClientSocket) {
    LspService::new(move |client| Backend {
        client,
        engine: Mutex::new(engine),
        docs: Mutex::default(),
        state: Mutex::default(),
    })
}

pub async fn serve_stdio(engine: Engine) {
    let (service, socket) = service(engine);
    Server::new(tokio::io::stdin(), tokio::io::stdout(), socket)
        .serve(service)
        .await;
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Byte de una posición LSP. Sin UTF-8 negociado, `character` cuenta unidades UTF-16.
pub fn to_offset(text: &str, pos: Position, utf8: bool) -> usize {
    let mut start = 0;
    for _ in 0..pos.line {
        match text[start..].find('\n') {
            Some(i) => start += i + 1,
            None => return text.len(),
        }
    }
    let line = &text[start..start + text[start..].find('\n').unwrap_or(text.len() - start)];
    let mut units = 0;
    for (i, ch) in line.char_indices() {
        if units >= pos.character as usize {
            return start + i;
        }
        units += if utf8 { ch.len_utf8() } else { ch.len_utf16() };
    }
    start + line.len()
}

pub fn to_position(text: &str, offset: usize, utf8: bool) -> Position {
    let before = &text[..offset.min(text.len())];
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    let col = &before[line_start..];
    let character = if utf8 {
        col.len()
    } else {
        col.encode_utf16().count()
    };
    Position {
        line: before.matches('\n').count() as u32,
        character: character as u32,
    }
}

fn file_path(uri: &Uri) -> Option<PathBuf> {
    uri.to_file_path().map(|p| p.into_owned())
}

impl Backend {
    fn utf8(&self) -> bool {
        lock(&self.state).utf8
    }

    async fn notify(&self, notices: Vec<Notice>) {
        for n in notices {
            match n {
                Notice::Warning(m) => self.client.show_message(MessageType::WARNING, m).await,
                Notice::Log(m) => self.client.log_message(MessageType::INFO, m).await,
            }
        }
    }

    fn root(&self) -> std::result::Result<PathBuf, String> {
        let root = lock(&self.state)
            .root
            .clone()
            .ok_or("molde: abre una carpeta de proyecto")?;
        Ok(lock(&self.engine).find_project(&root).unwrap_or(root))
    }

    async fn open(&self, path: &Path, line: u32) {
        let Some(uri) = Uri::from_file_path(path) else {
            return;
        };
        let at = Position { line, character: 0 };
        let params = ShowDocumentParams {
            uri,
            external: Some(false),
            take_focus: Some(true),
            selection: Some(Range { start: at, end: at }),
        };
        let _ = self.client.show_document(params).await;
    }

    /// La captura pregunta a la IA, si la hay, sin el motor bloqueado: la completion sigue respondiendo.
    async fn capture(&self, args: &[Value]) -> CommandResult {
        let uri: Uri = args
            .first()
            .and_then(Value::as_str)
            .ok_or("molde.capture: falta el documento")?
            .parse()
            .map_err(|_| "molde.capture: uri inválida")?;
        let line = |i: usize| args.get(i).and_then(Value::as_u64).unwrap_or(0) as usize;
        let path = file_path(&uri).ok_or("molde.capture: el documento no es un fichero")?;
        let (draft, assistant) = {
            let docs = lock(&self.docs);
            let doc = docs
                .get(&uri)
                .ok_or("molde.capture: el documento no está abierto")?;
            let engine = lock(&self.engine);
            let draft =
                engine.prepare_capture(&path, &doc.language, &doc.text, line(1), line(2))?;
            (draft, engine.assistant())
        };
        let Some(assistant) = assistant else {
            let c = lock(&self.engine).finish_capture(draft, None)?;
            return Ok((Some((c.file, c.line)), c.notices));
        };
        let progress = self.begin_progress("generando el snippet con IA").await;
        let prompt = draft.prompt.clone();
        let answer = ask(assistant, prompt).await;
        let captured = lock(&self.engine).finish_capture(draft, Some(answer));
        if let Some(p) = progress {
            p.finish().await;
        }
        let c = captured?;
        Ok((Some((c.file, c.line)), c.notices))
    }

    async fn begin_progress(
        &self,
        message: &str,
    ) -> Option<OngoingProgress<Unbounded, NotCancellable>> {
        let supported = lock(&self.state).progress;
        if !supported {
            return None;
        }
        let token = ProgressToken::Number(NEXT_PROGRESS.fetch_add(1, Ordering::Relaxed));
        self.client
            .create_work_done_progress(token.clone())
            .await
            .ok()?;
        Some(
            self.client
                .progress(token, "molde")
                .with_message(message)
                .begin()
                .await,
        )
    }

    /// Ejecuta un comando y devuelve el fichero que hay que abrir, si lo hay.
    fn run(&self, command: &str, args: &[Value]) -> CommandResult {
        let arg = |i: usize| args.get(i).and_then(Value::as_str).map(str::to_string);
        match command {
            "molde.recordUse" => {
                let a = args.first().ok_or("molde.recordUse: faltan argumentos")?;
                let field = |k: &str| {
                    a.get(k)
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string()
                };
                let layer =
                    Layer::parse(&field("layer")).ok_or("molde.recordUse: capa desconocida")?;
                let id = SnippetId {
                    layer,
                    scope: field("scope"),
                    language: field("language"),
                    key: field("key"),
                };
                lock(&self.engine).record_use(id, field("root"))?;
                Ok((None, Vec::new()))
            }
            "molde.stats" => {
                let root = self.root()?;
                let (rows, notices) = lock(&self.engine).stats(Some(&root))?;
                Ok((Some((report("stats", &stats_markdown(&rows))?, 0)), notices))
            }
            "molde.candidates" => {
                let root = self.root()?;
                let (candidates, notices) = lock(&self.engine).candidates(&root);
                Ok((
                    Some((report("candidates", &candidates_markdown(&candidates))?, 0)),
                    notices,
                ))
            }
            "molde.scaffold" => {
                let (template, name) = (
                    arg(0).ok_or("molde.scaffold: falta la plantilla")?,
                    arg(1).ok_or("molde.scaffold: falta el nombre")?,
                );
                let root = self.root()?;
                match lock(&self.engine).scaffold(&root, &template, &name, arg(2).as_deref()) {
                    Ok(done) => Ok((done.main.map(|m| (root.join(m), 0)), Vec::new())),
                    Err(ScaffoldError::Invalid(errors)) => {
                        Err(format!("molde scaffold: {}", errors.join("; ")))
                    }
                    Err(ScaffoldError::Post {
                        command, output, ..
                    }) => Err(format!("molde scaffold: falló \"{command}\":\n{output}")),
                }
            }
            other => Err(format!("molde: comando desconocido {other}")),
        }
    }
}

/// La IA puede tardar segundos: en un hilo aparte, para no parar el servidor.
async fn ask(assistant: Arc<dyn Assistant>, prompt: String) -> std::result::Result<String, String> {
    tokio::task::spawn_blocking(move || assistant.ask(&prompt))
        .await
        .unwrap_or_else(|e| Err(format!("la IA ha fallado: {e}")))
}

/// Escribe un informe en Markdown en un fichero temporal propio.
fn report(kind: &str, markdown: &str) -> std::result::Result<PathBuf, String> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let path = std::env::temp_dir().join(format!("molde-{kind}-{}-{nanos}.md", std::process::id()));
    std::fs::write(&path, markdown).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        let utf8 = params
            .capabilities
            .general
            .as_ref()
            .and_then(|g| g.position_encodings.as_ref())
            .is_some_and(|e| e.contains(&PositionEncodingKind::UTF8));
        #[allow(deprecated)]
        let root = params
            .workspace_folders
            .as_ref()
            .and_then(|f| f.first())
            .and_then(|f| file_path(&f.uri))
            .or_else(|| params.root_uri.as_ref().and_then(file_path));
        let progress = params
            .capabilities
            .window
            .as_ref()
            .and_then(|w| w.work_done_progress)
            .unwrap_or(false);
        *lock(&self.state) = State {
            utf8,
            root,
            progress,
        };
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                position_encoding: Some(if utf8 {
                    PositionEncodingKind::UTF8
                } else {
                    PositionEncodingKind::UTF16
                }),
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                // Sin resolve: las ediciones de imports van ya en el item.
                completion_provider: Some(CompletionOptions::default()),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: COMMANDS.map(String::from).to_vec(),
                    ..Default::default()
                }),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "molde".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
            ..Default::default()
        })
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let d = params.text_document;
        lock(&self.docs).insert(
            d.uri,
            Doc {
                text: d.text,
                language: d.language_id,
            },
        );
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last()
            && let Some(doc) = lock(&self.docs).get_mut(&params.text_document.uri)
        {
            doc.text = change.text;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        lock(&self.docs).remove(&params.text_document.uri);
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let TextDocumentPositionParams {
            text_document,
            position,
        } = params.text_document_position;
        let Some(path) = file_path(&text_document.uri) else {
            return Ok(None);
        };
        let utf8 = self.utf8();
        let (items, notices) = {
            let docs = lock(&self.docs);
            let Some(doc) = docs.get(&text_document.uri) else {
                return Ok(None);
            };
            let offset = to_offset(&doc.text, position, utf8);
            let req = CompletionRequest {
                path: &path,
                language: &doc.language,
                text: &doc.text,
                offset,
            };
            let (items, notices) = lock(&self.engine).complete(&req);
            // Todos los items comparten inicio y casi siempre punto de import: se calculan una vez.
            let mut positions: HashMap<usize, Position> = HashMap::new();
            let mut at = |offset: usize| {
                *positions
                    .entry(offset)
                    .or_insert_with(|| to_position(&doc.text, offset, utf8))
            };
            let items: Vec<CompletionItem> = items
                .into_iter()
                .map(|it| lsp_item(it, &mut at, position))
                .collect();
            (items, notices)
        };
        self.notify(notices).await;
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let r = params.range;
        // Una selección que acaba al principio de una línea no incluye esa línea.
        let last = if r.end.character == 0 && r.end.line > r.start.line {
            r.end.line - 1
        } else {
            r.end.line
        };
        if last <= r.start.line {
            return Ok(None);
        }
        let command = Command {
            title: CAPTURE_TITLE.into(),
            command: "molde.capture".into(),
            arguments: Some(vec![
                json!(params.text_document.uri.as_str()),
                json!(r.start.line),
                json!(last),
            ]),
        };
        let action = CodeAction {
            title: CAPTURE_TITLE.into(),
            kind: Some(CodeActionKind::REFACTOR_EXTRACT),
            command: Some(command),
            ..Default::default()
        };
        Ok(Some(vec![CodeActionOrCommand::CodeAction(action)]))
    }

    async fn execute_command(&self, params: ExecuteCommandParams) -> Result<Option<LSPAny>> {
        let result = if params.command == "molde.capture" {
            self.capture(&params.arguments).await
        } else {
            self.run(&params.command, &params.arguments)
        };
        match result {
            Ok((open, notices)) => {
                self.notify(notices).await;
                if let Some((path, line)) = open {
                    self.open(&path, line).await;
                }
            }
            Err(message) => self.client.show_message(MessageType::ERROR, message).await,
        }
        Ok(None)
    }
}

fn lsp_item(
    it: molde_core::CompletionItem,
    at: &mut impl FnMut(usize) -> Position,
    cursor: Position,
) -> CompletionItem {
    let start = at(it.replace_start);
    let additional = it.import_edit.as_ref().map(|(offset, new_text)| {
        let p = at(*offset);
        vec![TextEdit {
            range: Range { start: p, end: p },
            new_text: new_text.clone(),
        }]
    });
    let args = json!({
        "layer": it.id.layer.as_str(), "scope": it.id.scope, "language": it.id.language, "key": it.id.key, "root": it.project_root,
    });
    CompletionItem {
        label_details: Some(CompletionItemLabelDetails {
            detail: None,
            description: Some("molde".into()),
        }),
        kind: Some(CompletionItemKind::SNIPPET),
        detail: (!it.description.is_empty()).then(|| it.description.clone()),
        filter_text: Some(it.label.clone()),
        sort_text: Some(it.sort_text),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        text_edit: Some(CompletionTextEdit::Edit(TextEdit {
            range: Range { start, end: cursor },
            new_text: it.body,
        })),
        additional_text_edits: additional,
        command: Some(Command {
            title: "molde: registrar uso".into(),
            command: "molde.recordUse".into(),
            arguments: Some(vec![args]),
        }),
        label: it.label,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_roundtrip_in_both_encodings() {
        let text = "a\n# € 🧾x\n";
        let x = text.find('x').unwrap();
        for utf8 in [true, false] {
            let p = to_position(text, x, utf8);
            assert_eq!(to_offset(text, p, utf8), x);
        }
        assert_eq!(
            to_position(text, x, false),
            Position {
                line: 1,
                character: 6
            }
        );
        assert_eq!(
            to_position(text, x, true),
            Position {
                line: 1,
                character: 10
            }
        );
    }
}
