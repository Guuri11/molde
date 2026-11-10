//! 02_captura.feature: la regla de la captura con IA. Las IA son scripts falsos fuera del proyecto.

use crate::world::*;
use cucumber::gherkin::Step;
use cucumber::{given, then, when};
use serde_json::{Value, json};
use std::os::unix::fs::PermissionsExt;
use std::time::Instant;

const RUST: &str = "use std::sync::Arc;\n\npub fn crear(valor: u32) -> Arc<u32> {\n    let doble = valor * 2;\n    Arc::new(doble)\n}\n";
/// Filas de la selección en `RUST`, desde 0.
const RUST_SELECTION: (u32, u32) = (2, 5);

/// Un snippet que pasa la validación.
fn valid(key: &str) -> Value {
    json!({
        "key": key,
        "prefix": "con ia",
        "description": "Snippet de la IA",
        "body": ["let ${1:valor} = 1;$0"],
        "requires": ["use std::sync::Arc;"],
        "context": ["source_file"]
    })
}

fn fake_ai(w: &mut MoldeWorld, script: &str, secs: u64) {
    std::fs::create_dir_all(&w.ai_dir).unwrap();
    let path = w.ai_dir.join("ia.sh");
    std::fs::write(&path, format!("#!/bin/sh\n{script}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    w.ai = Some((path.display().to_string(), secs));
}

/// La respuesta se lee en cada llamada: cambiarla no obliga a reiniciar el servidor.
fn responds(w: &mut MoldeWorld, answer: &str) {
    std::fs::create_dir_all(&w.ai_dir).unwrap();
    let file = w.ai_dir.join("respuesta.txt");
    std::fs::write(&file, answer).unwrap();
    fake_ai(w, &format!("cat > /dev/null\ncat '{}'", file.display()), 5);
}

fn answer(w: &MoldeWorld) -> Value {
    serde_json::from_str(&std::fs::read_to_string(w.ai_dir.join("respuesta.txt")).unwrap()).unwrap()
}

fn lines(v: &Value) -> String {
    v.as_array()
        .unwrap()
        .iter()
        .map(|l| l.as_str().unwrap())
        .collect::<Vec<_>>()
        .join("\n")
}

#[given(regex = r"^MOLDE_CAPTURE_COMMAND con una IA que (.+)$")]
async fn with_ai(w: &mut MoldeWorld, what: String, step: &Step) {
    let dir = w.ai_dir.display().to_string();
    match what.as_str() {
        "responde:" => responds(w, step.docstring.as_deref().unwrap().trim()),
        w_ if w_.starts_with("responde un snippet válido con el import") => {
            let [import, context] = <[String; 2]>::try_from(quoted(w_)).unwrap();
            let mut s = valid("con-descartes");
            s["requires"] = json!(["use std::sync::Arc;", import]);
            s["context"] = json!(["source_file", context]);
            responds(w, &s.to_string());
        }
        w_ if w_.starts_with("responde la clave") => {
            responds(w, &valid(&quoted(w_)[0]).to_string())
        }
        "no existe" => w.ai = Some((format!("{dir}/no-existe"), 5)),
        "termina con código de salida 1" => {
            fake_ai(w, "cat > /dev/null\necho 'sin crédito' >&2\nexit 1", 5)
        }
        "tarda más que MOLDE_CAPTURE_TIMEOUT" => fake_ai(w, "cat > /dev/null\nsleep 10", 1),
        "responde un texto sin ningún objeto JSON" => responds(w, "No sé hacer eso."),
        "responde un JSON sin \"body\"" => responds(w, r#"{"key": "sin-cuerpo", "prefix": "x"}"#),
        "responde un cuerpo con un placeholder sin cerrar" => {
            let mut s = valid("sin-cerrar");
            s["body"] = json!(["let ${1:valor = 1;"]);
            responds(w, &s.to_string());
        }
        "guarda lo que recibe y la carpeta donde se ejecuta" => fake_ai(
            w,
            &format!(
                "cat > '{dir}/recibido.txt'\npwd > '{dir}/carpeta.txt'\nls -A > '{dir}/contenido.txt'\necho '{}'",
                valid("recibido")
            ),
            5,
        ),
        "tarda 2 segundos en responder" => {
            fake_ai(
                w,
                &format!("cat > /dev/null\nsleep 2\necho '{}'", valid("lenta")),
                5,
            );
            w.ai_slow = true;
        }
        other => panic!("IA desconocida: {other}"),
    }
}

/// Selecciona `rows` del documento abierto y ejecuta la acción de captura, como el editor.
async fn capture_rows(w: &mut MoldeWorld, (first, last): (u32, u32)) {
    std::fs::create_dir_all(w.path(".molde")).unwrap();
    let doc = w.doc.clone().expect("documento abierto");
    let slow = w.ai_slow;
    let lsp = w.lsp().await;
    let res = lsp
        .request("textDocument/codeAction", json!({ "textDocument": { "uri": doc.uri }, "range": { "start": { "line": first, "character": 0 }, "end": { "line": last + 1, "character": 0 } }, "context": { "diagnostics": [] } }))
        .await;
    let command = res["result"][0]["command"].clone();
    let id = lsp
        .send_request(
            "workspace/executeCommand",
            json!({ "command": command["command"], "arguments": command["arguments"] }),
        )
        .await;
    let mut during = None;
    if slow {
        let t = Instant::now();
        let position = crate::lsp_client::pos(&doc.text, doc.cursor, lsp.utf8);
        lsp.request(
            "textDocument/completion",
            json!({ "textDocument": { "uri": doc.uri }, "position": position }),
        )
        .await;
        during = Some((t.elapsed().as_millis(), !lsp.answered(id).await));
    }
    let res = lsp.response(id).await;
    assert!(res.get("error").is_none(), "{res}");
    w.during_capture = during;
    w.scratch = json!({ "file": w.project_snippets("rust") });
    w.captured = Some(w.last_entry("rust"));
    let text: Vec<&str> = doc.text.lines().collect();
    w.selection = Some((
        "rust".into(),
        text[first as usize..=last as usize].join("\n"),
    ));
}

#[when("guardo una selección de Rust como snippet")]
async fn capture_rust(w: &mut MoldeWorld) {
    if w.doc.is_none() {
        w.open("src/caso.rs", "rust", RUST).await;
    }
    capture_rows(w, RUST_SELECTION).await;
}

fn captured(w: &MoldeWorld) -> &molde_core::Snippet {
    w.captured.as_ref().expect("entrada capturada")
}

#[then(
    regex = r#"^la entrada nueva es "(.+)" con el prefijo, la descripción, el cuerpo, "requires" y "context" que ha respondido la IA$"#
)]
async fn entry_is_answer(w: &mut MoldeWorld, key: String) {
    let a = answer(w);
    let s = captured(w);
    assert_eq!(s.key, key);
    assert_eq!(s.prefixes, [a["prefix"].as_str().unwrap()]);
    assert_eq!(s.description, a["description"]);
    assert_eq!(s.body, lines(&a["body"]));
    assert_eq!(json!(s.requires), a["requires"]);
    assert_eq!(json!(s.context), a["context"]);
}

#[then("la entrada se guarda con el cuerpo que ha respondido la IA")]
async fn body_is_answer(w: &mut MoldeWorld) {
    assert_eq!(captured(w).body, lines(&answer(w)["body"]));
}

#[then(regex = r#"^"requires" no contiene "(.+)"$"#)]
async fn requires_without(w: &mut MoldeWorld, import: String) {
    assert!(
        !captured(w).requires.contains(&import),
        "{captured:?}",
        captured = captured(w)
    );
    assert!(
        !captured(w).requires.is_empty(),
        "se ha quitado también lo válido"
    );
}

#[then(regex = r#"^"context" no contiene "(.+)", que .+$"#)]
async fn context_without(w: &mut MoldeWorld, kind: String) {
    let context = captured(w).context.clone().expect("context");
    assert_eq!(context, ["source_file"], "sin {kind}");
}

async fn warnings(w: &mut MoldeWorld) -> Vec<String> {
    w.lsp()
        .await
        .messages("window/showMessage")
        .await
        .iter()
        .map(|m| m["message"].as_str().unwrap().to_string())
        .collect()
}

#[then("molde muestra un aviso con lo que ha descartado")]
async fn warns_dropped(w: &mut MoldeWorld) {
    let shown = warnings(w).await;
    assert!(
        shown
            .iter()
            .any(|m| m.contains("use serde::{") && m.contains("class_body")),
        "{shown:?}"
    );
}

#[given(regex = r#"^"(.+)" con la clave "(.+)"$"#)]
async fn file_with_key(w: &mut MoldeWorld, file: String, key: String) {
    w.add_snippet(&file, &key, axum_get_by_id());
}

#[then(regex = r#"^la entrada nueva tiene la clave "(.+)"$"#)]
async fn new_key(w: &mut MoldeWorld, key: String) {
    assert_eq!(captured(w).key, key);
}

#[then(regex = r#"^la entrada "(.+)" que ya existía no cambia$"#)]
async fn existing_unchanged(w: &mut MoldeWorld, key: String) {
    let map: serde_json::Map<String, Value> =
        serde_json::from_str(&w.read(&w.project_snippets("rust"))).unwrap();
    assert_eq!(map[&key], axum_get_by_id());
}

#[when(regex = r#"^la IA responde la clave "(.+)"$"#)]
async fn answers_key(w: &mut MoldeWorld, key: String) {
    responds(w, &valid(&key).to_string());
    capture_rows(w, RUST_SELECTION).await;
}

#[then(regex = r#"^la entrada nueva también tiene una clave "captura-N"$"#)]
async fn provisional_again(w: &mut MoldeWorld) {
    assert_eq!(captured(w).key, "captura-2");
}

#[then(regex = r#"^la entrada se guarda con la generalización por regla y la clave "(.+)"$"#)]
async fn rule_capture(w: &mut MoldeWorld, key: String) {
    let (lang, sel) = w.selection.clone().unwrap();
    let rule = w.engine().generalize(&lang, &sel, &[]);
    assert_eq!(captured(w).key, key);
    assert_eq!(captured(w).body, rule);
}

#[then("molde muestra un aviso con el motivo")]
async fn warns_reason(w: &mut MoldeWorld) {
    let shown = warnings(w).await;
    assert!(
        shown
            .iter()
            .any(|m| m.contains("la IA no ha dado un snippet válido (")
                && m.contains("se ha capturado por regla")),
        "{shown:?}"
    );
}

const OTHER_IMPORTS: [&str; 4] = [
    "use std::collections::HashMap;",
    "use serde::Serialize;",
    "use chrono::Utc;",
    "use tracing::info;",
];
const USED_IMPORTS: [&str; 2] = ["use std::sync::Arc;", "use uuid::Uuid;"];
const FIVE: [&str; 5] = [
    "pub fn crear(id: Uuid) -> Arc<String> {",
    "    let texto = id.to_string();",
    "    let valor = Arc::new(texto);",
    "    valor",
    "}",
];
const FIVE_AT: u32 = 100;

#[given("un fichero Rust de 200 líneas con 6 imports")]
async fn long_file(w: &mut MoldeWorld) {
    let mut text: Vec<String> = USED_IMPORTS
        .iter()
        .chain(OTHER_IMPORTS.iter())
        .map(|s| s.to_string())
        .collect();
    while text.len() < 200 {
        if text.len() == FIVE_AT as usize {
            text.extend(FIVE.iter().map(|s| s.to_string()));
        } else {
            text.push(format!("fn relleno_{}() {{}}", text.len()));
        }
    }
    w.open("src/largo.rs", "rust", &(text.join("\n") + "\n"))
        .await;
}

#[when("guardo como snippet 5 líneas que usan 2 de esos imports")]
async fn capture_five(w: &mut MoldeWorld) {
    capture_rows(w, (FIVE_AT, FIVE_AT + 4)).await;
}

fn received(w: &MoldeWorld, file: &str) -> String {
    std::fs::read_to_string(w.ai_dir.join(file)).unwrap()
}

#[then("la IA ha recibido el lenguaje, esas 5 líneas y esos 2 imports")]
async fn got_selection(w: &mut MoldeWorld) {
    let prompt = received(w, "recibido.txt");
    for part in ["rust"]
        .iter()
        .chain(FIVE.iter())
        .chain(USED_IMPORTS.iter())
    {
        assert!(prompt.contains(part), "falta {part}:\n{prompt}");
    }
}

#[then("ninguna otra línea del fichero")]
async fn nothing_else(w: &mut MoldeWorld) {
    let prompt = received(w, "recibido.txt");
    assert!(!prompt.contains("relleno_"), "{prompt}");
    for import in OTHER_IMPORTS {
        assert!(!prompt.contains(import), "{import}:\n{prompt}");
    }
}

#[then("el comando se ha ejecutado en una carpeta temporal vacía, fuera del proyecto")]
async fn ran_in_empty_temp(w: &mut MoldeWorld) {
    let dir = std::path::PathBuf::from(received(w, "carpeta.txt").trim());
    assert!(dir.starts_with(std::env::temp_dir()), "{}", dir.display());
    assert!(!dir.starts_with(&w.project), "{}", dir.display());
    assert_eq!(received(w, "contenido.txt"), "");
    assert!(!dir.exists(), "no se ha borrado {}", dir.display());
}

#[then("el editor recibe un progreso de molde hasta que la entrada está guardada")]
async fn progress(w: &mut MoldeWorld) {
    let lsp = w.lsp().await;
    assert!(
        !lsp.messages("window/workDoneProgress/create")
            .await
            .is_empty()
    );
    let kinds: Vec<Value> = lsp
        .messages("$/progress")
        .await
        .iter()
        .map(|p| p["value"].clone())
        .collect();
    assert_eq!(kinds.len(), 2, "{kinds:?}");
    assert_eq!(kinds[0]["kind"], "begin");
    assert_eq!(kinds[0]["title"], "molde");
    assert_eq!(kinds[1]["kind"], "end");
}

#[then("una completion pedida durante la espera responde sin esperar a la IA")]
async fn completion_during(w: &mut MoldeWorld) {
    let (ms, pending) = w.during_capture.expect("completion durante la captura");
    assert!(pending, "la captura ya había terminado");
    assert!(ms < 1000, "la completion ha tardado {ms} ms");
}
