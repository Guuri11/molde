//! Snippets en VSCode JSON y uso en SQLite, más los adaptadores de ficheros, comandos, IA y reloj.

use molde_core::*;
use rusqlite::{Connection, params};
use serde_json::{Map, Value};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Ficheros `<lenguaje>.json` con el formato de VSCode más `requires` y `context`.
/// Admite comentarios y comas finales, como VSCode.
pub struct JsonSnippets;

impl SnippetRepository for JsonSnippets {
    fn load(&self, path: &Path) -> Result<Option<Vec<Snippet>>, SnippetFileError> {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
            Err(e) => {
                return Err(SnippetFileError {
                    line: 0,
                    message: e.to_string(),
                });
            }
        };
        let map = parse(&text)?;
        let mut out = Vec::new();
        for (key, value) in &map {
            let snippet = to_snippet(key, value).map_err(|message| SnippetFileError {
                line: line_of_key(&text, key),
                message: format!("{key}: {message}"),
            })?;
            out.push(snippet);
        }
        Ok(Some(out))
    }

    fn append(&self, path: &Path, snippet: &Snippet) -> Result<u32, String> {
        let entry = render_entry(snippet);
        let (content, line) = match std::fs::read_to_string(path) {
            Err(e) if e.kind() == ErrorKind::NotFound => (format!("{{\n{entry}\n}}\n"), 1),
            Err(e) => return Err(format!("{}: {e}", path.display())),
            // Se inserta como texto antes de la llave final para no tocar el formato ni los comentarios.
            Ok(text) => {
                let map = parse(&text)
                    .map_err(|e| format!("{}:{}: {}", path.display(), e.line, e.message))?;
                let clean = strip_jsonc(&text);
                let close = clean
                    .rfind('}')
                    .ok_or_else(|| format!("{}: no es un objeto JSON", path.display()))?;
                let before = clean[..close].trim_end().len();
                let mut out = text[..before].to_string();
                if !map.is_empty() && !clean[..before].ends_with(',') {
                    out.push(',');
                }
                out.push('\n');
                let line = out.matches('\n').count() as u32;
                out.push_str(&entry);
                let rest = &text[before..];
                if rest.starts_with('}') {
                    out.push('\n');
                }
                out.push_str(rest);
                (out, line)
            }
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::write(path, content).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(line)
    }

    fn parse_entry(&self, text: &str) -> Result<Snippet, String> {
        // El primer `{` desde el que empieza un objeto JSON: la IA puede añadir texto alrededor.
        let mut first_error = None;
        let value = text
            .match_indices('{')
            .find_map(|(i, _)| {
                let parsed = serde_json::Deserializer::from_str(&text[i..])
                    .into_iter::<Value>()
                    .next()?;
                parsed
                    .map_err(|e| first_error.get_or_insert(e.to_string()).clone())
                    .ok()
            })
            .ok_or_else(|| match first_error {
                Some(e) => format!("el JSON no es válido: {e}"),
                None => "la respuesta no tiene ningún objeto JSON".to_string(),
            })?;
        let key = value.get("key").and_then(Value::as_str).unwrap_or_default();
        fields(key, &value)
    }
}

fn parse(text: &str) -> Result<Map<String, Value>, SnippetFileError> {
    serde_json::from_str(&strip_jsonc(text)).map_err(|e| SnippetFileError {
        line: e.line(),
        message: e
            .to_string()
            .split(" at line ")
            .next()
            .unwrap_or_default()
            .to_string(),
    })
}

/// Quita comentarios y comas finales cambiándolos por espacios: las posiciones no se mueven,
/// así que las líneas de los errores siguen siendo las del fichero.
fn strip_jsonc(text: &str) -> String {
    let b = text.as_bytes();
    let mut out = b.to_vec();
    let (mut i, mut in_str) = (0, false);
    while i < b.len() {
        let c = b[i];
        if in_str {
            match c {
                b'\\' => i += 1,
                b'"' => in_str = false,
                _ => {}
            }
            i += 1;
            continue;
        }
        match (c, b.get(i + 1)) {
            (b'"', _) => in_str = true,
            (b'/', Some(b'/')) => {
                while i < b.len() && b[i] != b'\n' {
                    out[i] = b' ';
                    i += 1;
                }
                continue;
            }
            (b'/', Some(b'*')) => {
                let end = text[i + 2..].find("*/").map_or(b.len(), |e| i + 2 + e + 2);
                for o in &mut out[i..end] {
                    if *o != b'\n' {
                        *o = b' ';
                    }
                }
                i = end;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    // Comas finales, ya sin comentarios.
    let (mut i, mut in_str) = (0, false);
    while i < out.len() {
        match out[i] {
            b'\\' if in_str => i += 1,
            b'"' => in_str = !in_str,
            b',' if !in_str => {
                let next = out[i + 1..].iter().find(|c| !c.is_ascii_whitespace());
                if matches!(next, Some(b'}' | b']')) {
                    out[i] = b' ';
                }
            }
            _ => {}
        }
        i += 1;
    }
    String::from_utf8(out).unwrap_or_default()
}

fn line_of_key(text: &str, key: &str) -> usize {
    let needle = serde_json::to_string(key).unwrap_or_default();
    text.find(&needle)
        .map_or(0, |i| text[..i].matches('\n').count() + 1)
}

fn strings(o: &Map<String, Value>, field: &str) -> Result<Option<Vec<String>>, String> {
    match o.get(field) {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(vec![s.clone()])),
        Some(Value::Array(a)) => a
            .iter()
            .map(|v| {
                v.as_str()
                    .map(String::from)
                    .ok_or_else(|| format!("\"{field}\" solo admite textos"))
            })
            .collect::<Result<_, _>>()
            .map(Some),
        Some(_) => Err(format!(
            "\"{field}\" debe ser un texto o una lista de textos"
        )),
    }
}

fn to_snippet(key: &str, value: &Value) -> Result<Snippet, String> {
    let s = fields(key, value)?;
    if s.prefixes.is_empty() {
        return Err("falta \"prefix\"".into());
    }
    Ok(s)
}

/// Los campos de una entrada, sin exigir el prefijo.
fn fields(key: &str, value: &Value) -> Result<Snippet, String> {
    let o = value.as_object().ok_or("debe ser un objeto")?;
    Ok(Snippet {
        key: key.to_string(),
        prefixes: strings(o, "prefix")?.unwrap_or_default(),
        description: strings(o, "description")?.unwrap_or_default().join("\n"),
        body: strings(o, "body")?.ok_or("falta \"body\"")?.join("\n"),
        requires: strings(o, "requires")?.unwrap_or_default(),
        context: strings(o, "context")?,
    })
}

fn render_entry(s: &Snippet) -> String {
    let mut m = Map::new();
    let prefix = match s.prefixes.as_slice() {
        [one] => Value::from(one.clone()),
        many => Value::from(many.to_vec()),
    };
    m.insert("prefix".into(), prefix);
    m.insert("description".into(), Value::from(s.description.clone()));
    m.insert(
        "body".into(),
        Value::from(s.body.split('\n').collect::<Vec<_>>()),
    );
    if !s.requires.is_empty() {
        m.insert("requires".into(), Value::from(s.requires.clone()));
    }
    if let Some(c) = &s.context {
        m.insert("context".into(), Value::from(c.clone()));
    }
    let pretty = serde_json::to_string_pretty(&Value::Object(m)).unwrap_or_default();
    format!(
        "  {}: {}",
        serde_json::to_string(&s.key).unwrap_or_default(),
        pretty.replace('\n', "\n  ")
    )
}

/// `~/.local/share/molde/usage.sqlite`
pub struct SqliteUsage {
    conn: Connection,
}

impl SqliteUsage {
    pub fn open(path: &Path) -> Result<Self, String> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let conn = Connection::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        // El servidor LSP y la CLI pueden escribir a la vez.
        conn.busy_timeout(Duration::from_secs(5))
            .map_err(|e| e.to_string())?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS uses (layer TEXT NOT NULL, scope TEXT NOT NULL, language TEXT NOT NULL,
                 key TEXT NOT NULL, project_root TEXT NOT NULL, used_at INTEGER NOT NULL);
             CREATE INDEX IF NOT EXISTS uses_by_snippet ON uses (layer, scope, language, key);
             CREATE TABLE IF NOT EXISTS snippets (layer TEXT NOT NULL, scope TEXT NOT NULL, language TEXT NOT NULL,
                 key TEXT NOT NULL, first_seen INTEGER NOT NULL, PRIMARY KEY (layer, scope, language, key));",
        )
        .map_err(|e| e.to_string())?;
        Ok(SqliteUsage { conn })
    }
}

impl UsageStore for SqliteUsage {
    fn record(&mut self, e: &UsageEvent) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO uses VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    e.id.layer.as_str(),
                    e.id.scope,
                    e.id.language,
                    e.id.key,
                    e.project_root,
                    e.at
                ],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    fn register(&mut self, ids: &[SnippetId], at: i64) -> Result<(), String> {
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        {
            let mut st = tx
                .prepare_cached("INSERT OR IGNORE INTO snippets VALUES (?1, ?2, ?3, ?4, ?5)")
                .map_err(|e| e.to_string())?;
            for id in ids {
                st.execute(params![
                    id.layer.as_str(),
                    id.scope,
                    id.language,
                    id.key,
                    at
                ])
                .map_err(|e| e.to_string())?;
            }
        }
        tx.commit().map_err(|e| e.to_string())
    }

    fn stats(&self, ids: &[SnippetId]) -> Result<Vec<UsageStats>, String> {
        let err = |e: rusqlite::Error| e.to_string();
        let mut uses = self
            .conn
            .prepare_cached("SELECT COUNT(*), MAX(used_at) FROM uses WHERE layer = ?1 AND scope = ?2 AND language = ?3 AND key = ?4")
            .map_err(err)?;
        let mut seen = self
            .conn
            .prepare_cached("SELECT first_seen FROM snippets WHERE layer = ?1 AND scope = ?2 AND language = ?3 AND key = ?4")
            .map_err(err)?;
        ids.iter()
            .map(|id| {
                let p = params![id.layer.as_str(), id.scope, id.language, id.key];
                let (count, last): (i64, Option<i64>) = uses
                    .query_row(p, |r| Ok((r.get(0)?, r.get(1)?)))
                    .map_err(err)?;
                let first_seen: Option<i64> = seen.query_row(p, |r| r.get(0)).ok();
                Ok(UsageStats {
                    first_seen,
                    uses: count as u64,
                    last_use: last,
                })
            })
            .collect()
    }
}

pub struct StdFs;

impl FileSystem for StdFs {
    fn read(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    fn write(&self, path: &Path, content: &[u8]) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, content)
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn stamp(&self, path: &Path) -> Option<Stamp> {
        let m = std::fs::metadata(path).ok()?;
        Some((m.modified().ok()?, m.len()))
    }

    fn walk(&self, dir: &Path) -> Vec<PathBuf> {
        if !dir.is_dir() {
            return Vec::new();
        }
        let mut files: Vec<PathBuf> = ignore::WalkBuilder::new(dir)
            .require_git(false)
            .build()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_some_and(|t| t.is_file()))
            .map(ignore::DirEntry::into_path)
            .collect();
        files.sort();
        files
    }
}

pub struct StdShell;

impl Shell for StdShell {
    fn run(&self, command: &str, cwd: &Path) -> (bool, String) {
        match Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(cwd)
            .stdin(Stdio::null())
            .output()
        {
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

/// La IA de `MOLDE_CAPTURE_COMMAND`: el prompt entra por la entrada estándar y la respuesta sale
/// por la salida estándar. molde no sabe qué IA hay detrás.
pub struct CommandAssistant {
    pub command: String,
    pub timeout: Duration,
}

impl CommandAssistant {
    /// None si `MOLDE_CAPTURE_COMMAND` no está o está vacía. `MOLDE_CAPTURE_TIMEOUT` va en segundos.
    pub fn from_vars(var: impl Fn(&str) -> Option<String>) -> Option<CommandAssistant> {
        let command = var("MOLDE_CAPTURE_COMMAND").filter(|c| !c.trim().is_empty())?;
        let secs = var("MOLDE_CAPTURE_TIMEOUT")
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(120);
        Some(CommandAssistant {
            command,
            timeout: Duration::from_secs(secs),
        })
    }
}

impl Assistant for CommandAssistant {
    fn ask(&self, prompt: &str) -> Result<String, String> {
        // Carpeta vacía: un agente no carga las instrucciones del proyecto ni puede tocarlo.
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let dir = std::env::temp_dir().join(format!("molde-ia-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let result = run_with_input(&self.command, &dir, prompt, self.timeout);
        let _ = std::fs::remove_dir_all(&dir);
        result
    }
}

/// `sh -c` con `input` por la entrada estándar. Devuelve la salida estándar si termina bien a tiempo.
fn run_with_input(
    command: &str,
    cwd: &Path,
    input: &str,
    timeout: Duration,
) -> Result<String, String> {
    use std::io::{Read, Write};
    use std::os::unix::process::CommandExt;
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Grupo propio, para terminar también lo que lance el comando si se pasa de tiempo.
        .process_group(0)
        .spawn()
        .map_err(|e| format!("\"{command}\": {e}"))?;
    let (mut stdin, mut stdout, mut stderr) = (
        child.stdin.take().expect("stdin con pipe"),
        child.stdout.take().expect("stdout con pipe"),
        child.stderr.take().expect("stderr con pipe"),
    );
    // Escribir y leer en hilos: con una salida grande, el comando se bloquearía esperando.
    let input = input.to_string();
    std::thread::spawn(move || stdin.write_all(input.as_bytes()));
    let out = std::thread::spawn(move || {
        let mut b = Vec::new();
        let _ = stdout.read_to_end(&mut b);
        b
    });
    let err = std::thread::spawn(move || {
        let mut b = Vec::new();
        let _ = stderr.read_to_end(&mut b);
        b
    });
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            Ok(None) => {
                let _ = Command::new("kill")
                    .args(["-KILL", &format!("-{}", child.id())])
                    .status();
                let _ = child.wait();
                return Err(format!("no ha respondido en {} s", timeout.as_secs()));
            }
            Err(e) => return Err(e.to_string()),
        }
    };
    if !status.success() {
        let code = status
            .code()
            .map_or("una señal".to_string(), |c| format!("el código {c}"));
        let err = String::from_utf8_lossy(&err.join().unwrap_or_default()).to_string();
        let last = err.trim().lines().last().unwrap_or_default().to_string();
        return Err(format!("\"{command}\" ha terminado con {code}: {last}"));
    }
    Ok(String::from_utf8_lossy(&out.join().unwrap_or_default()).into_owned())
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs() as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jsonc_keeps_positions_and_append_keeps_comments() {
        let text = "{\n  // comentario\n  \"a\": { \"prefix\": \"a\", \"body\": [\"x\",], },\n}\n";
        assert_eq!(strip_jsonc(text).len(), text.len());
        let dir = std::env::temp_dir().join(format!("molde-store-{}", std::process::id()));
        let path = dir.join("rust.json");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, text).unwrap();
        let s = Snippet {
            key: "b".into(),
            prefixes: vec!["b".into()],
            body: "y\n$0".into(),
            ..Default::default()
        };
        let line = JsonSnippets.append(&path, &s).unwrap();
        let written = std::fs::read_to_string(&path).unwrap();
        assert!(written.contains("// comentario"));
        assert_eq!(written.lines().nth(line as usize).unwrap(), "  \"b\": {");
        let keys: Vec<String> = JsonSnippets
            .load(&path)
            .unwrap()
            .unwrap()
            .into_iter()
            .map(|s| s.key)
            .collect();
        assert_eq!(keys, ["a", "b"]);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
