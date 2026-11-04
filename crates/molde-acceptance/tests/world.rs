//! Estado de cada escenario: un home y un proyecto temporales, el motor y una sesión LSP.

use crate::lsp_client::{Lsp, pos, uri};
use cucumber::World;
use molde_core::*;
use molde_store::{JsonSnippets, SqliteUsage, StdFs};
use molde_syntax::Treesitter;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Marca del cursor en los textos de prueba.
pub const CURSOR: char = '‸';
pub const DAY: i64 = 86_400;

#[derive(Clone, Debug)]
pub struct Doc {
    pub uri: String,
    pub language: String,
    pub text: String,
    pub cursor: usize,
}

#[derive(World)]
#[world(init = Self::new)]
pub struct MoldeWorld {
    // Se borra al acabar el escenario.
    _tmp: tempfile::TempDir,
    pub home: PathBuf,
    pub project: PathBuf,
    pub now: i64,
    pub engine: Option<Engine>,
    /// Consultas de Go para el escenario de añadir un lenguaje.
    pub go_queries: Option<(String, String)>,
    pub lsp: Option<Lsp>,
    pub utf8: bool,
    pub pending: Option<(String, String, String)>,
    pub doc: Option<Doc>,
    pub results: Vec<Vec<Value>>,
    pub accepted: Option<Value>,
    pub applied: Option<String>,
    pub snippet_lang: String,
    pub last_key: String,
    pub selection: Option<(String, String)>,
    pub file_prefix: String,
    pub captured: Option<Snippet>,
    pub body: String,
    pub cli: (i32, String, String),
    pub snapshot: BTreeMap<PathBuf, Vec<u8>>,
    pub scratch: Value,
}

impl std::fmt::Debug for MoldeWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MoldeWorld({})", self.project.display())
    }
}

/// Ejecuta en un hilo bloqueante sin parar el resto de escenarios.
pub fn blocking<T>(f: impl FnOnce() -> T) -> T {
    tokio::task::block_in_place(f)
}

/// `sh -c` con `bin/` del proyecto delante en el PATH, para los sustitutos de herramientas.
struct PathShell(PathBuf);

impl Shell for PathShell {
    fn run(&self, command: &str, cwd: &Path) -> (bool, String) {
        let path = format!(
            "{}:{}",
            self.0.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let out = std::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(cwd)
            .env("PATH", path)
            .stdin(std::process::Stdio::null())
            .output();
        match out {
            Ok(o) => (
                o.status.success(),
                format!(
                    "{}{}",
                    String::from_utf8_lossy(&o.stdout),
                    String::from_utf8_lossy(&o.stderr)
                ),
            ),
            Err(e) => (false, e.to_string()),
        }
    }
}

struct FixedClock(i64);

impl Clock for FixedClock {
    fn now(&self) -> i64 {
        self.0
    }
}

pub fn language_id(name: &str) -> &'static str {
    match name {
        "Rust" | "rust" => "rust",
        "Python" | "python" => "python",
        "TypeScript" | "typescript" => "typescript",
        "\".tsx\"" | "TSX" | "typescriptreact" => "typescriptreact",
        "Go" | "go" => "go",
        other => panic!("lenguaje desconocido: {other}"),
    }
}

pub fn extension(language: &str) -> &'static str {
    match language {
        "rust" => "rs",
        "python" => "py",
        "typescript" => "ts",
        "typescriptreact" => "tsx",
        "go" => "go",
        other => panic!("lenguaje desconocido: {other}"),
    }
}

/// Textos entre comillas simples o dobles: `"a", "b" y 'c'` → [a, b, c].
pub fn quoted(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '"' || c == '\'' {
            out.push(chars.by_ref().take_while(|x| *x != c).collect());
        }
    }
    out
}

pub fn key_of(item: &Value) -> &str {
    item["command"]["arguments"][0]["key"]
        .as_str()
        .unwrap_or_default()
}

pub fn keys(items: &[Value]) -> Vec<&str> {
    items.iter().map(key_of).collect()
}

pub fn snip(prefix: &str, body: &str) -> Value {
    json!({ "prefix": prefix, "body": body.split('\n').collect::<Vec<_>>() })
}

/// El snippet de ejemplo del README.
pub fn axum_get_by_id() -> Value {
    json!({
        "prefix": "axum handler get by id",
        "description": "Handler GET por id con servicio y DTO",
        "body": [
            "pub async fn get_${1:invoice}(",
            "    State(svc): State<Arc<${2:Invoice}Service>>,",
            "    Path(id): Path<Uuid>,",
            ") -> Result<Json<${2}Dto>, ApiError> {",
            "    let ${1} = svc.find(id).await?;",
            "    Ok(Json(${1}.into()))",
            "}$0"
        ],
        "requires": ["use std::sync::Arc;", "use axum::extract::{Path, State};", "use axum::Json;", "use uuid::Uuid;"],
        "context": ["source_file", "mod_item"]
    })
}

impl MoldeWorld {
    fn new() -> Self {
        let tmp = tempfile::Builder::new()
            .prefix("molde-")
            .tempdir()
            .expect("directorio temporal");
        let (home, project) = (tmp.path().join("home"), tmp.path().join("proyecto"));
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&project).unwrap();
        MoldeWorld {
            _tmp: tmp,
            home,
            project,
            now: 1_793_750_400,
            engine: None,
            go_queries: None,
            lsp: None,
            utf8: true,
            pending: None,
            doc: None,
            results: Vec::new(),
            accepted: None,
            applied: None,
            snippet_lang: "rust".into(),
            last_key: String::new(),
            selection: None,
            file_prefix: String::new(),
            captured: None,
            body: String::new(),
            cli: (0, String::new(), String::new()),
            snapshot: BTreeMap::new(),
            scratch: Value::Null,
        }
    }

    pub fn paths(&self) -> molde_cli::Paths {
        let home = self.home.display().to_string();
        molde_cli::Paths::from_vars(|k| (k == "HOME").then(|| home.clone()))
    }

    pub fn syntax(&self) -> Treesitter {
        let t = Treesitter::new();
        match &self.go_queries {
            Some((containers, imports)) => t
                .with_language(
                    "go",
                    &["go"],
                    tree_sitter_go::LANGUAGE.into(),
                    containers,
                    imports,
                )
                .expect("consultas de Go"),
            None => t,
        }
    }

    pub fn build_engine(&self) -> Engine {
        let paths = self.paths();
        let ports = Ports {
            snippets: Box::new(JsonSnippets),
            usage: Box::new(SqliteUsage::open(&paths.usage_db).expect("base de datos de usos")),
            syntax: Box::new(self.syntax()),
            clock: Box::new(FixedClock(self.now)),
            fs: Box::new(StdFs),
            shell: Box::new(PathShell(self.project.join("bin"))),
        };
        Engine::new(ports, paths.user_dir)
    }

    pub fn engine(&mut self) -> &mut Engine {
        if self.engine.is_none() {
            self.engine = Some(self.build_engine());
        }
        self.engine.as_mut().unwrap()
    }

    pub fn usage(&self) -> SqliteUsage {
        SqliteUsage::open(&self.paths().usage_db).unwrap()
    }

    /// `~/...` va al home de prueba; el resto, al proyecto.
    pub fn path(&self, p: &str) -> PathBuf {
        match p.strip_prefix("~/") {
            Some(rest) => self.home.join(rest),
            None => self.project.join(p),
        }
    }

    pub fn write(&self, p: &str, content: &str) {
        let path = self.path(p);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    pub fn read(&self, p: &str) -> String {
        std::fs::read_to_string(self.path(p)).unwrap_or_else(|e| panic!("{p}: {e}"))
    }

    pub fn add_snippet(&mut self, file: &str, key: &str, value: Value) {
        let path = self.path(file);
        let mut map: Map<String, Value> = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        map.insert(key.to_string(), value);
        self.write(file, &serde_json::to_string_pretty(&map).unwrap());
        self.last_key = key.to_string();
    }

    pub fn project_snippets(&self, language: &str) -> String {
        format!(".molde/snippets/{language}.json")
    }

    pub fn id(&self, language: &str, key: &str) -> SnippetId {
        SnippetId {
            layer: Layer::Project,
            scope: self.project.display().to_string(),
            language: language.into(),
            key: key.into(),
        }
    }

    pub async fn lsp(&mut self) -> &mut Lsp {
        if self.lsp.is_none() {
            let engine = self.build_engine();
            self.lsp = Some(Lsp::start(engine, &self.project, self.utf8).await);
        }
        self.lsp.as_mut().unwrap()
    }

    /// Abre un documento en el servidor; `‸` marca el cursor.
    pub async fn open(&mut self, rel: &str, language: &str, text: &str) {
        let cursor = text.find(CURSOR).unwrap_or(text.len());
        let text = text.replace(CURSOR, "");
        self.write(rel, &text);
        let path = self.path(rel);
        let doc = Doc {
            uri: uri(&path),
            language: language.into(),
            text,
            cursor,
        };
        let params = json!({ "textDocument": { "uri": doc.uri, "languageId": doc.language, "version": 1, "text": doc.text } });
        self.lsp()
            .await
            .notify("textDocument/didOpen", params)
            .await;
        self.doc = Some(doc);
    }

    pub async fn change(&mut self, text: &str, version: i64) {
        let mut doc = self.doc.clone().expect("documento abierto");
        doc.cursor = text.find(CURSOR).unwrap_or(text.len());
        doc.text = text.replace(CURSOR, "");
        let params = json!({ "textDocument": { "uri": doc.uri, "version": version }, "contentChanges": [{ "text": doc.text }] });
        self.lsp()
            .await
            .notify("textDocument/didChange", params)
            .await;
        self.doc = Some(doc);
    }

    /// Abre el documento pendiente o uno por defecto del lenguaje de los snippets.
    pub async fn ensure_doc(&mut self) {
        if self.doc.is_some() {
            return;
        }
        let (rel, lang, text) = self.pending.take().unwrap_or_else(|| {
            let lang = self.snippet_lang.clone();
            let text = match lang.as_str() {
                "python" => "def main():\n    pass\n\n‸\n",
                "typescript" => "const a = 1;\n‸\n",
                "typescriptreact" => "export const A = () => <div />;\n‸\n",
                _ => "fn main() {}\n\n‸\n",
            };
            (
                format!("src/main.{}", extension(&lang)),
                lang,
                text.to_string(),
            )
        });
        self.open(&rel, &lang, &text).await;
    }

    pub async fn complete(&mut self) -> Vec<Value> {
        self.ensure_doc().await;
        let doc = self.doc.clone().unwrap();
        let lsp = self.lsp().await;
        let p = pos(&doc.text, doc.cursor, lsp.utf8);
        let res = lsp
            .request(
                "textDocument/completion",
                json!({ "textDocument": { "uri": doc.uri }, "position": p }),
            )
            .await;
        let items = res["result"].as_array().cloned().unwrap_or_default();
        self.results.push(items.clone());
        items
    }

    pub fn last(&self) -> &[Value] {
        self.results.last().map(Vec::as_slice).unwrap_or_default()
    }

    pub fn item(&self, key: &str) -> Value {
        self.last()
            .iter()
            .find(|i| key_of(i) == key)
            .cloned()
            .unwrap_or_else(|| panic!("no aparece {key}: {:?}", keys(self.last())))
    }

    /// Pide completion y acepta el item de `key`, como haría el editor salvo ejecutar su comando.
    pub async fn accept(&mut self, key: &str) -> Value {
        self.complete().await;
        let item = self.item(key);
        let doc = self.doc.clone().unwrap();
        self.applied = Some(crate::lsp_client::apply(
            &doc.text,
            &item,
            self.lsp().await.utf8,
        ));
        self.accepted = Some(item.clone());
        item
    }

    pub fn applied_lines(&self) -> Vec<String> {
        self.applied
            .as_deref()
            .expect("snippet aceptado")
            .lines()
            .map(String::from)
            .collect()
    }

    /// Ficheros del proyecto y su contenido, sin `target/`.
    pub fn take_snapshot(&self) -> BTreeMap<PathBuf, Vec<u8>> {
        let mut out = BTreeMap::new();
        let mut stack = vec![self.project.clone()];
        while let Some(dir) = stack.pop() {
            for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
                let p = e.path();
                if p.is_dir() {
                    if p.file_name().is_some_and(|n| n != "target") {
                        stack.push(p);
                    }
                } else {
                    out.insert(
                        p.strip_prefix(&self.project).unwrap().to_path_buf(),
                        std::fs::read(&p).unwrap_or_default(),
                    );
                }
            }
        }
        out
    }

    pub fn run_cli(&mut self, args: &str) {
        self.snapshot = self.take_snapshot();
        let args: Vec<String> = args.split_whitespace().map(String::from).collect();
        let project = self.project.clone();
        let mut engine = self.build_engine();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = blocking(|| molde_cli::run(&args, &project, &mut engine, &mut out, &mut err));
        self.cli = (
            code,
            String::from_utf8_lossy(&out).into(),
            String::from_utf8_lossy(&err).into(),
        );
    }

    pub async fn shown(&mut self) -> Vec<Value> {
        self.lsp().await.messages("window/showDocument").await
    }
}
