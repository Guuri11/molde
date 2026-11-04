//! 02_captura.feature

use crate::lsp_client::uri;
use crate::world::*;
use cucumber::gherkin::Step;
use cucumber::{given, then, when};
use molde_core::body;
use serde_json::{Value, json};

const TS: &str = "import { db } from \"./db\";\n\nexport async function total(id: string) {\n  const rows = await db.query(id);\n  return rows.length;\n}\n";

#[given("un fichero TypeScript abierto")]
async fn ts_open(w: &mut MoldeWorld) {
    w.open("src/total.ts", "typescript", TS).await;
}

#[when(regex = r"^pido acciones de código con una selección de (una línea|\d+ líneas)$")]
async fn code_actions(w: &mut MoldeWorld, size: String) {
    let n: u32 = if size == "una línea" {
        1
    } else {
        size.split(' ').next().unwrap().parse().unwrap()
    };
    let uri = w.doc.clone().unwrap().uri;
    let (start, end) = (
        json!({ "line": 2, "character": 0 }),
        json!({ "line": 2 + n - 1, "character": 10 }),
    );
    let res = w
        .lsp()
        .await
        .request("textDocument/codeAction", json!({ "textDocument": { "uri": uri }, "range": { "start": start, "end": end }, "context": { "diagnostics": [] } }))
        .await;
    w.scratch = res["result"].clone();
}

fn titles(w: &MoldeWorld) -> Vec<String> {
    w.scratch
        .as_array()
        .into_iter()
        .flatten()
        .map(|a| a["title"].as_str().unwrap().to_string())
        .collect()
}

#[then(regex = r#"^aparece "(.+)"$"#)]
async fn action_appears(w: &mut MoldeWorld, title: String) {
    assert!(titles(w).contains(&title), "{:?}", titles(w));
}

#[then(regex = r#"^no aparece "(.+)"$"#)]
async fn action_missing(w: &mut MoldeWorld, title: String) {
    assert!(!titles(w).contains(&title));
}

#[when("guardo una selección de Python como snippet")]
async fn capture_python(w: &mut MoldeWorld) {
    std::fs::create_dir_all(w.path(".molde")).unwrap();
    let text = "from pathlib import Path\n\ndef leer(nombre):\n    ruta = Path(nombre)\n    return ruta.read_text()\n";
    w.open("app/leer.py", "python", text).await;
    let uri = w.doc.clone().unwrap().uri;
    let res = w
        .lsp()
        .await
        .request("textDocument/codeAction", json!({ "textDocument": { "uri": uri }, "range": { "start": { "line": 2, "character": 0 }, "end": { "line": 5, "character": 0 } }, "context": { "diagnostics": [] } }))
        .await;
    let command = res["result"][0]["command"].clone();
    let lsp = w.lsp().await;
    lsp.exec(
        command["command"].as_str().unwrap(),
        command["arguments"].clone(),
    )
    .await;
}

fn entries(w: &MoldeWorld, file: &str) -> serde_json::Map<String, Value> {
    serde_json::from_str(&w.read(file)).expect("JSON válido")
}

#[then(regex = r#"^la entrada nueva se añade a "(.+)" con una clave única$"#)]
async fn entry_added(w: &mut MoldeWorld, file: String) {
    let map = entries(w, &file);
    let keys: Vec<&String> = map.keys().filter(|k| k.starts_with("captura-")).collect();
    assert_eq!(keys, ["captura-1"]);
    w.scratch = json!({ "file": file, "key": keys[0] });
}

#[then("lleva un prefijo provisional y la descripción vacía")]
async fn provisional(w: &mut MoldeWorld) {
    let map = entries(w, w.scratch["file"].as_str().unwrap());
    let entry = &map["captura-1"];
    assert!(!entry["prefix"].as_str().unwrap().is_empty());
    assert_eq!(entry["description"], "");
    assert_eq!(entry["requires"], json!(["from pathlib import Path"]));
}

#[then("molde pide al editor que abra el fichero en esa entrada")]
async fn opens_entry(w: &mut MoldeWorld) {
    let file = w.scratch["file"].as_str().unwrap().to_string();
    let shown = w.shown().await;
    assert_eq!(shown.len(), 1, "{shown:?}");
    assert_eq!(shown[0]["uri"], uri(&w.path(&file)));
    let line = shown[0]["selection"]["start"]["line"].as_u64().unwrap() as usize;
    assert_eq!(
        w.read(&file).lines().nth(line).unwrap().trim(),
        "\"captura-1\": {"
    );
}

#[given(regex = r"^la selección de (\w+):$")]
async fn selection(w: &mut MoldeWorld, lang: String, step: &Step) {
    w.selection = Some((
        language_id(&lang).into(),
        step.docstring
            .clone()
            .unwrap()
            .trim_matches('\n')
            .to_string(),
    ));
}

#[when("la guardo como snippet")]
async fn save_selection(w: &mut MoldeWorld) {
    let (lang, sel) = w.selection.clone().expect("selección");
    std::fs::create_dir_all(w.path(".molde")).unwrap();
    let text = format!("{}{sel}\n", w.file_prefix);
    let rel = format!("src/seleccion.{}", extension(&lang));
    w.write(&rel, &text);
    let first = w.file_prefix.lines().count();
    let last = first + sel.lines().count() - 1;
    let path = w.path(&rel);
    let c = w
        .engine()
        .capture(&path, &lang, &text, first, last)
        .expect("captura");
    w.body = c.snippet.body.clone();
    w.captured = Some(c.snippet);
}

/// El cuerpo sin placeholders ni tabstops: lo que queda fijo.
fn fixed_text(body: &str) -> String {
    let (mut out, mut chars) = (String::new(), body.chars().peekable());
    while let Some(c) = chars.next() {
        match c {
            '\\' => out.extend(chars.next()),
            '$' if chars.peek() == Some(&'{') => {
                let mut depth = 0;
                for x in chars.by_ref() {
                    match x {
                        '{' => depth += 1,
                        '}' if depth == 1 => break,
                        '}' => depth -= 1,
                        _ => {}
                    }
                }
            }
            '$' if chars.peek().is_some_and(char::is_ascii_digit) => {
                while chars.peek().is_some_and(char::is_ascii_digit) {
                    chars.next();
                }
            }
            c => out.push(c),
        }
    }
    out
}

#[then(regex = r#"^todas las (?:apariciones )?de "(.+)" pasan a ser el placeholder (\d+)$"#)]
async fn term_is_placeholder(w: &mut MoldeWorld, term: String, n: u32) {
    assert!(w.body.contains(&format!("${{{n}:{term}}}")), "{}", w.body);
    assert!(
        !fixed_text(&w.body).contains(&term),
        "queda {term} fijo en {}",
        w.body
    );
    let expanded = body::expand(&w.body).unwrap();
    assert_eq!(expanded, w.selection.as_ref().unwrap().1);
}

#[then(regex = r"^(.+) se quedan como están$")]
async fn stay_fixed(w: &mut MoldeWorld, names: String) {
    let fixed = fixed_text(&w.body);
    for name in quoted(&names) {
        assert!(fixed.contains(&name), "{name} no está fijo en {}", w.body);
    }
}

#[then(regex = r#"^el cuerpo termina con el tabstop final "(.+)"$"#)]
async fn ends_with_tabstop(w: &mut MoldeWorld, tabstop: String) {
    assert!(w.body.ends_with(&tabstop), "{}", w.body);
}

#[then(regex = r"^(.+) pasan a ser tabstops con su valor original como texto por defecto$")]
async fn literals_are_tabstops(w: &mut MoldeWorld, values: String) {
    let values: Vec<String> = values
        .replace(" y ", ", ")
        .split(", ")
        .map(|v| v.trim_matches('"').to_string())
        .collect();
    for v in values {
        let at = w
            .body
            .find(&format!(":{v}}}"))
            .unwrap_or_else(|| panic!("{v} no es tabstop en {}", w.body));
        let head = &w.body[..at];
        let open = head.rfind("${").unwrap();
        assert!(
            head[open + 2..].chars().all(|c| c.is_ascii_digit()),
            "{}",
            w.body
        );
    }
}

#[given(regex = r#"^"\.molde/molde\.toml" con (.+)$"#)]
async fn molde_toml(w: &mut MoldeWorld, content: String) {
    w.write(".molde/molde.toml", &format!("{content}\n"));
}

#[given(regex = r#"^una selección donde "(.+)" aparece en "(.+)" y en "(.+)"$"#)]
async fn selection_with_term(w: &mut MoldeWorld, _term: String, a: String, b: String) {
    w.selection = Some((
        "python".into(),
        format!("{a} = cargar({b})\nprint({a}.nombre, {b})"),
    ));
}

#[then(regex = r#"^"(.+)" no se convierte en placeholder$"#)]
async fn not_placeholder(w: &mut MoldeWorld, term: String) {
    assert!(
        !w.body.contains(&format!(":{term}}}")) && !w.body.contains(&format!(":{term}_")),
        "{}",
        w.body
    );
    assert!(
        fixed_text(&w.body).contains(&format!("current_{term}")),
        "{}",
        w.body
    );
}

/// Ventanas de código real de los tres lenguajes: el propio código de molde y los fixtures.
fn real_fragments() -> Vec<(String, String)> {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
    let mut files: Vec<(String, std::path::PathBuf)> = Vec::new();
    for e in std::fs::read_dir(root).unwrap().flatten() {
        let src = e.path().join("src");
        let mut stack = vec![src];
        while let Some(dir) = stack.pop() {
            for f in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
                let p = f.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x == "rs") {
                    files.push(("rust".into(), p));
                }
            }
        }
    }
    for f in std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures"))
        .unwrap()
        .flatten()
    {
        let p = f.path();
        match p.extension().and_then(|x| x.to_str()) {
            Some("ts") => files.push(("typescript".into(), p)),
            Some("tsx") => files.push(("typescriptreact".into(), p)),
            Some("py") => files.push(("python".into(), p)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for (lang, path) in files {
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        for start in (0..lines.len()).step_by(4) {
            out.push((
                lang.clone(),
                dedent(&lines[start..(start + 12).min(lines.len())]),
            ));
        }
    }
    out
}

fn dedent(lines: &[&str]) -> String {
    let indent = lines
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    lines
        .iter()
        .map(|l| l.get(indent..).unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

#[given("cualquier selección guardada como snippet")]
async fn any_selection(w: &mut MoldeWorld) {
    let fragments = real_fragments();
    assert!(
        fragments.len() > 500,
        "pocos fragmentos: {}",
        fragments.len()
    );
    let engine = w.engine();
    let saved: Vec<Value> = fragments
        .into_iter()
        .map(|(lang, sel)| json!({ "sel": sel, "body": engine.generalize(&lang, &sel, &[]) }))
        .collect();
    w.scratch = json!(saved);
}

#[when("expando el snippet sin cambiar ningún placeholder")]
async fn expand_all(w: &mut MoldeWorld) {
    for s in w.scratch.as_array_mut().unwrap() {
        let expanded = body::expand(s["body"].as_str().unwrap()).expect("cuerpo válido");
        s["expanded"] = Value::from(expanded);
    }
}

#[then("el texto insertado es idéntico a la selección original")]
async fn roundtrip(w: &mut MoldeWorld) {
    let all = w.scratch.as_array().unwrap();
    let placeholders = all
        .iter()
        .filter(|s| s["body"].as_str().unwrap().contains("${1:"))
        .count();
    for s in all {
        assert_eq!(s["expanded"], s["sel"], "cuerpo: {}", s["body"]);
    }
    assert!(
        placeholders * 2 > all.len(),
        "solo {placeholders} de {} fragmentos tienen placeholders",
        all.len()
    );
}

#[then(regex = r#"^los "\$" y "\}" del template literal quedan escapados en el cuerpo$"#)]
async fn escaped(w: &mut MoldeWorld) {
    assert!(
        w.body.contains("\\${base\\}") && w.body.contains("\\${id\\}"),
        "{}",
        w.body
    );
}

#[then("expandirlo reproduce la línea original")]
async fn expands_back(w: &mut MoldeWorld) {
    assert_eq!(
        body::expand(&w.body).unwrap(),
        w.selection.as_ref().unwrap().1
    );
}

#[given(regex = r#"^un fichero (\w+) con (['"].*)$"#)]
async fn file_with_imports(w: &mut MoldeWorld, lang: String, imports: String) {
    w.snippet_lang = language_id(&lang).into();
    w.file_prefix = quoted(&imports).join("\n") + "\n\n";
}

#[given(regex = r"^una selección que (?:solo )?usa (.+)$")]
async fn selection_using(w: &mut MoldeWorld, names: String) {
    let lang = w.snippet_lang.clone();
    let lines: Vec<String> = quoted(&names)
        .iter()
        .enumerate()
        .map(|(i, n)| {
            if lang == "python" {
                format!("v{i} = {n}(\"x\")")
            } else {
                format!("const v{i} = {n}.cargar();")
            }
        })
        .chain([if lang == "python" {
            "print(v0)".to_string()
        } else {
            "console.log(v0);".to_string()
        }])
        .collect();
    w.selection = Some((lang, lines.join("\n")));
}

#[then(regex = r#"^"requires" contiene el import de "(.+)"$"#)]
async fn requires_contains(w: &mut MoldeWorld, module: String) {
    let requires = &w.captured.as_ref().unwrap().requires;
    assert!(
        requires
            .iter()
            .any(|r| r.contains(&format!("\"{module}\""))),
        "{requires:?}"
    );
}

#[then(regex = r#"^no contiene el de "(.+)", porque .+$"#)]
async fn requires_not_contains(w: &mut MoldeWorld, module: String) {
    let requires = &w.captured.as_ref().unwrap().requires;
    assert!(
        !requires.iter().any(|r| r.contains(&module)),
        "{requires:?}"
    );
    w.scratch = json!(module);
}

#[then("la descripción provisional avisa del import omitido para revisarlo")]
async fn description_warns(w: &mut MoldeWorld) {
    let description = &w.captured.as_ref().unwrap().description;
    assert!(
        description.contains(w.scratch.as_str().unwrap()),
        "{description}"
    );
}

#[then(regex = r#"^"requires" contiene solo "(.+)"$"#)]
async fn requires_only(w: &mut MoldeWorld, import: String) {
    assert_eq!(w.captured.as_ref().unwrap().requires, [import]);
}

#[given("que el servidor está en marcha")]
async fn server_running(w: &mut MoldeWorld) {
    w.add_snippet(
        &w.project_snippets("python"),
        "v1",
        snip("version uno", "uno$0"),
    );
    w.open("app/main.py", "python", "x = 1\n‸\n").await;
    assert_eq!(keys(&w.complete().await), ["v1"]);
}

#[when(regex = r#"^edito "(.+)" desde cualquier programa y lo guardo$"#)]
async fn edit_externally(w: &mut MoldeWorld, file: String) {
    let content =
        serde_json::to_string_pretty(&json!({ "v2": snip("version dos", "dos$0") })).unwrap();
    std::fs::write(w.path(&file), content).unwrap();
}

#[then("la siguiente completion ya usa la versión nueva")]
async fn uses_new_version(w: &mut MoldeWorld) {
    assert_eq!(keys(&w.complete().await), ["v2"]);
}

#[given(regex = r#"^que guardo "(.+)" con un error de sintaxis en la línea (\d+)$"#)]
async fn invalid_file(w: &mut MoldeWorld, file: String, line: usize) {
    w.add_snippet(&file, "valido", snip("valido", "version buena$0"));
    w.open("src/main.rs", "rust", "fn main() {}\n‸\n").await;
    assert_eq!(keys(&w.complete().await), ["valido"]);
    let mut lines = vec![
        "{".to_string(),
        "  \"valido\": {".into(),
        "    \"prefix\": \"valido\",".into(),
        "    \"body\": [".into(),
    ];
    while lines.len() < line - 1 {
        lines.push("      \"linea\",".into());
    }
    lines.extend([
        "      \"falta una coma\" \"aquí\"".into(),
        "    ]".into(),
        "  }".into(),
        "}".into(),
    ]);
    std::fs::write(w.path(&file), lines.join("\n")).unwrap();
    w.scratch = json!({ "file": file, "line": line });
}

#[when("molde intenta recargarlo")]
async fn reload(w: &mut MoldeWorld) {
    w.complete().await;
}

#[then("sigue usando la última versión válida de ese fichero")]
async fn keeps_last_valid(w: &mut MoldeWorld) {
    assert_eq!(w.item("valido")["textEdit"]["newText"], "version buena$0");
}

#[then("muestra un aviso con el fichero y la línea del error")]
async fn warns_with_line(w: &mut MoldeWorld) {
    let shown = w.lsp().await.messages("window/showMessage").await;
    let expected = format!(
        "{}:{}:",
        w.path(w.scratch["file"].as_str().unwrap()).display(),
        w.scratch["line"]
    );
    assert!(
        shown
            .iter()
            .any(|m| m["message"].as_str().unwrap().starts_with(&expected)),
        "esperaba {expected}: {shown:?}"
    );
}
