//! 05_casos_limite.feature

use crate::lsp_client::{Lsp, apply, pos};
use crate::world::*;
use cucumber::{given, then, when};
use molde_core::{Context, SyntaxAnalyzer};
use serde_json::{Value, json};
use std::time::Instant;

// Posiciones UTF-16 frente a bytes

#[given(regex = r#"^una línea Python con el comentario "(.+)" antes del cursor$"#)]
async fn line_with_comment(w: &mut MoldeWorld, comment: String) {
    let file = w.project_snippets("python");
    w.add_snippet(&file, "en-funcion", json!({ "prefix": "f", "body": ["BODY$0"], "context": ["function_definition"], "requires": ["from decimal import Decimal"] }));
    w.add_snippet(
        &file,
        "en-modulo",
        json!({ "prefix": "m", "body": ["m$0"], "context": ["module"] }),
    );
    w.scratch = json!(comment);
}

#[when("pido completion al final de esa línea")]
async fn complete_end_of_line(w: &mut MoldeWorld) {
    let comment = w.scratch.as_str().unwrap().to_string();
    let text = format!("import os\n\ndef total():\n    x = 1  {comment}");
    let mut runs = Vec::new();
    for utf8 in [true, false] {
        let mut lsp = Lsp::start(w.build_engine(), &w.project, utf8).await;
        w.write("app/total.py", &text);
        let uri = crate::lsp_client::uri(&w.path("app/total.py"));
        lsp.notify("textDocument/didOpen", json!({ "textDocument": { "uri": uri, "languageId": "python", "version": 1, "text": text } })).await;
        let cursor = pos(&text, text.len(), utf8);
        let res = lsp
            .request(
                "textDocument/completion",
                json!({ "textDocument": { "uri": uri }, "position": cursor }),
            )
            .await;
        runs.push(json!({ "utf8": utf8, "cursor": cursor, "items": res["result"] }));
    }
    w.scratch = json!({ "text": text, "runs": runs });
}

#[then("molde localiza el nodo correcto para decidir el contexto")]
async fn right_node(w: &mut MoldeWorld) {
    for run in w.scratch["runs"].as_array().unwrap() {
        let items = run["items"].as_array().unwrap();
        assert_eq!(keys(items), ["en-funcion"], "utf8={}", run["utf8"]);
    }
}

#[then("las ediciones que devuelve no cortan ni desplazan caracteres")]
async fn edits_intact(w: &mut MoldeWorld) {
    let text = w.scratch["text"].as_str().unwrap().to_string();
    for run in w.scratch["runs"].as_array().unwrap() {
        let item = &run["items"][0];
        assert_eq!(item["textEdit"]["range"]["start"], run["cursor"]);
        assert_eq!(item["textEdit"]["range"]["end"], run["cursor"]);
        let applied = apply(&text, item, run["utf8"].as_bool().unwrap());
        let expected =
            text.replacen("import os\n", "import os\nfrom decimal import Decimal\n", 1) + "BODY";
        assert_eq!(applied, expected, "utf8={}", run["utf8"]);
    }
}

// Código a medio escribir

const UNCLOSED: &str = "class A {\n  total() {\n    if (x) {\n      ahg‸\n}\n";

#[given("un fichero TypeScript con una llave sin cerrar justo antes del cursor")]
async fn unclosed_brace(w: &mut MoldeWorld) {
    let file = w.project_snippets("typescript");
    w.add_snippet(
        &file,
        "solo-clase",
        json!({ "prefix": "clase", "body": ["c$0"], "context": ["class_body"] }),
    );
    w.add_snippet(&file, "libre", snip("libre", "l$0"));
    w.pending = Some(("src/a.ts".into(), "typescript".into(), UNCLOSED.into()));
}

fn container(text: &str) -> Context {
    let cursor = text.find(CURSOR).unwrap();
    molde_syntax::Treesitter::new().container("typescript", &text.replace(CURSOR, ""), cursor)
}

#[then("molde busca el contexto en el primer ancestro sin error")]
async fn first_ancestor_without_error(_w: &mut MoldeWorld) {
    // El error de la línea que se escribe no cuenta: decide el contenedor que hay por encima.
    assert_eq!(
        container("class A {\n  total = 0;\n  ahg(‸\n}\n"),
        Context::Known("class_body".into())
    );
    // Aquí la llave sin cerrar deja todo el fichero dentro de un error de varias líneas.
    assert_eq!(container(UNCLOSED), Context::Unknown);
}

#[then("si no puede decidirlo ofrece los snippets igualmente, con menor prioridad")]
async fn undecided_lower_priority(w: &mut MoldeWorld) {
    let (restricted, free) = (w.item("solo-clase"), w.item("libre"));
    assert!(
        restricted["sortText"].as_str() > free["sortText"].as_str(),
        "{} {}",
        restricted["sortText"],
        free["sortText"]
    );
}

#[given("un fichero de más de 2 MB, umbral configurable con max_file_size_mb")]
async fn big_file(w: &mut MoldeWorld) {
    w.add_snippet(
        &w.project_snippets("python"),
        "solo-funcion",
        json!({ "prefix": "f", "body": ["f$0"], "context": ["function_definition"] }),
    );
    let block = "def f():\n    x = 1\n";
    let text = block.repeat(2 * 1024 * 1024 / block.len() + 10) + "ahg‸\n";
    w.pending = Some(("app/grande.py".into(), "python".into(), text));
}

#[then("molde no lo analiza con Treesitter y ofrece los snippets sin filtrar por contexto")]
async fn big_file_unfiltered(w: &mut MoldeWorld) {
    assert!(keys(w.last()).contains(&"solo-funcion"));
    // Con un umbral mayor sí se analiza, y en el nivel superior no toca.
    w.write(".molde/molde.toml", "max_file_size_mb = 10\n");
    assert!(!keys(&w.complete().await).contains(&"solo-funcion"));
}

// Dónde van los imports

fn header_fixture(lang: &str, header: &str) -> (&'static str, &'static str, usize) {
    match (lang, header) {
        ("python", "un shebang y un docstring de módulo") => (
            "#!/usr/bin/env python\n\"\"\"Módulo de ejemplo.\"\"\"\n\nreq‸\n",
            "from decimal import Decimal",
            2,
        ),
        ("python", "\"from __future__ import annotations\"") => (
            "from __future__ import annotations\n\nreq‸\n",
            "from decimal import Decimal",
            1,
        ),
        ("typescript", "la directiva \"use client\"") => {
            ("\"use client\";\n\nreq‸\n", "import { z } from \"zod\";", 1)
        }
        ("rust", "comentarios \"//!\" y atributos \"#![...]\"") => (
            "//! Documentación del crate.\n#![allow(dead_code)]\n\nreq‸\n",
            "use std::sync::Arc;",
            2,
        ),
        other => panic!("sin fixture para {other:?}"),
    }
}

#[given(regex = r"^un fichero (\w+) que empieza por (.+) y no tiene el import requerido$")]
async fn file_with_header(w: &mut MoldeWorld, lang: String, header: String) {
    let lang = language_id(&lang);
    let (text, import, lines) = header_fixture(lang, &header);
    w.add_snippet(
        &w.project_snippets(lang),
        "req",
        json!({ "prefix": "req", "body": ["usa()$0"], "requires": [import] }),
    );
    w.pending = Some((
        format!("src/a.{}", extension(lang)),
        lang.into(),
        text.into(),
    ));
    w.scratch = json!({ "import": import, "lines": lines, "header": text.lines().take(lines).collect::<Vec<_>>() });
}

#[when("acepto un snippet que lo requiere")]
async fn accept_requiring(w: &mut MoldeWorld) {
    w.accept("req").await;
}

#[then(regex = r"^el import se inserta después de (.+)$")]
async fn import_after_header(w: &mut MoldeWorld, _header: String) {
    let lines = w.applied_lines();
    let n = w.scratch["lines"].as_u64().unwrap() as usize;
    let header: Vec<String> = serde_json::from_value(w.scratch["header"].clone()).unwrap();
    assert_eq!(lines[..n], header[..], "{lines:?}");
    assert_eq!(lines[n], w.scratch["import"].as_str().unwrap(), "{lines:?}");
}

#[given(regex = r#"^el cursor dentro de "mod tests" en un fichero Rust$"#)]
async fn inside_mod_tests(w: &mut MoldeWorld) {
    let text = "use std::io;\n\npub fn a() {}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n\n    req‸\n}\n";
    w.pending = Some(("src/lib.rs".into(), "rust".into(), text.into()));
}

#[when(regex = r#"^acepto un snippet que requiere "(.+)"$"#)]
async fn accept_requiring_this(w: &mut MoldeWorld, import: String) {
    w.add_snippet(
        &w.project_snippets("rust"),
        "req",
        json!({ "prefix": "req", "body": ["#[rstest]$0"], "requires": [import] }),
    );
    w.accept("req").await;
}

#[then(regex = r#"^el import se añade dentro de "mod tests", no al principio del fichero$"#)]
async fn import_inside_mod(w: &mut MoldeWorld) {
    let lines = w.applied_lines();
    let at = lines.iter().position(|l| l == "    use super::*;").unwrap();
    assert_eq!(lines[at + 1], "    use rstest::rstest;", "{lines:?}");
    assert_eq!(lines[0], "use std::io;");
    assert!(!lines.contains(&"use rstest::rstest;".to_string()));
}

#[when("molde devuelve un snippet con imports")]
async fn snippet_with_imports(w: &mut MoldeWorld) {
    w.add_snippet(
        &w.project_snippets("rust"),
        "req",
        json!({ "prefix": "req", "body": ["Arc::new($1)$0"], "requires": ["use std::sync::Arc;"] }),
    );
    w.complete().await;
}

#[then("la edición del import va ya en el item de completion")]
async fn import_in_item(w: &mut MoldeWorld) {
    assert_eq!(
        w.item("req")["additionalTextEdits"][0]["newText"],
        "use std::sync::Arc;\n\n"
    );
}

#[then(regex = r#"^no depende de que el cliente pida "completionItem/resolve"$"#)]
async fn no_resolve(w: &mut MoldeWorld) {
    let provider = w.lsp().await.capabilities["completionProvider"].clone();
    assert!(provider.is_object(), "{provider}");
    assert!(provider["resolveProvider"].is_null() || provider["resolveProvider"] == false);
}

// Clientes LSP

#[when("el cliente inicializa molde")]
async fn initialize(w: &mut MoldeWorld) {
    w.lsp().await;
}

#[then(regex = r"^la respuesta declara (.+) como comandos$")]
async fn declares_commands(w: &mut MoldeWorld, commands: String) {
    let declared = w.lsp().await.capabilities["executeCommandProvider"]["commands"].clone();
    for c in quoted(&commands) {
        assert!(
            declared
                .as_array()
                .unwrap()
                .contains(&Value::from(c.as_str())),
            "falta {c}: {declared}"
        );
    }
}

#[given("un editor que inserta los snippets pero no ejecuta su comando")]
async fn editor_without_command(w: &mut MoldeWorld) {
    w.add_snippet(
        &w.project_snippets("rust"),
        "uso",
        snip("uso", "insertado(${1:x})$0"),
    );
}

#[when("acepto snippets en ese editor")]
async fn accept_without_command(w: &mut MoldeWorld) {
    for _ in 0..3 {
        w.accept("uso").await;
    }
}

#[then("los snippets funcionan con normalidad")]
async fn snippets_work(w: &mut MoldeWorld) {
    assert!(w.applied.as_deref().unwrap().contains("insertado(x)"));
}

fn uses(w: &mut MoldeWorld, key: &str) -> u64 {
    let project = w.project.clone();
    let mut engine = w.build_engine();
    let (rows, _) = engine.stats(Some(&project)).unwrap();
    rows.iter().find(|r| r.id.key == key).map_or(0, |r| r.uses)
}

#[then("esos usos no se registran")]
async fn uses_not_recorded(w: &mut MoldeWorld) {
    assert_eq!(uses(w, "uso"), 0);
}

#[then(regex = r#"^molde no los estima a partir de "completionItem/resolve", .+$"#)]
async fn not_from_resolve(w: &mut MoldeWorld) {
    let item = w.item("uso");
    w.lsp().await.request("completionItem/resolve", item).await;
    assert_eq!(uses(w, "uso"), 0);
}

#[given(
    "que no todos los clientes aplican las transformaciones de placeholders de la sintaxis de VSCode"
)]
async fn no_transforms(_w: &mut MoldeWorld) {}

#[when(regex = r#"^molde generaliza un término que aparece como "(.+)" y como "(.+)"$"#)]
async fn generalize_variants(w: &mut MoldeWorld, _lower: String, _upper: String) {
    let code = "pub async fn get_invoice(\n    State(svc): State<Arc<InvoiceService>>,\n) -> Json<InvoiceDto> {\n    let invoice = svc.find().await;\n    Json(invoice.into())\n}";
    w.body = w.engine().generalize("rust", code, &[]);
}

#[then("usa dos placeholders independientes en lugar de una transformación")]
async fn two_placeholders(w: &mut MoldeWorld) {
    assert!(
        w.body.contains("${1:invoice}") && w.body.contains("${2:Invoice}"),
        "{}",
        w.body
    );
    assert!(
        !w.body.contains("${1/") && !w.body.contains("${2/"),
        "{}",
        w.body
    );
}

// Monorepos y latencia

#[given(regex = r#"^un repositorio con "(.+)" y "(.+)"$"#)]
async fn monorepo(w: &mut MoldeWorld, a: String, b: String) {
    for dir in [a, b] {
        let name = dir.split('/').nth(1).unwrap().to_string();
        w.add_snippet(
            &format!("{dir}/snippets/rust.json"),
            &name,
            snip(&name, "x$0"),
        );
    }
}

#[when(regex = r#"^abro un fichero de "(.+)"$"#)]
async fn open_in(w: &mut MoldeWorld, dir: String) {
    w.open(&format!("{dir}/src/main.rs"), "rust", "fn main() {}\n‸\n")
        .await;
    w.complete().await;
}

#[then(regex = r#"^molde usa la carpeta "\.molde" más cercana subiendo desde ese fichero$"#)]
async fn nearest_molde(w: &mut MoldeWorld) {
    assert_eq!(keys(w.last()), ["api"]);
}

#[given(regex = r"^(\d+) snippets cargados entre todas las capas$")]
async fn many_snippets(w: &mut MoldeWorld, n: usize) {
    for (layer, file) in [
        ("u", "~/.config/molde/snippets/rust.json"),
        ("p", ".molde/snippets/rust.json"),
    ] {
        let mut map = serde_json::Map::new();
        for i in 0..n / 2 {
            let mut s = json!({ "prefix": format!("{layer} snippet {i}"), "body": [format!("fn s{i}() {{ ${{1:x}} }}$0")] });
            if i % 3 == 0 {
                s["requires"] = json!(["use std::sync::Arc;", "use axum::{extract::Path, Json};"]);
            }
            if i % 5 == 0 {
                s["context"] = json!(["source_file", "function_item"]);
            }
            map.insert(format!("{layer}-{i}"), s);
        }
        w.write(file, &serde_json::to_string(&map).unwrap());
    }
}

#[when(regex = r"^pido completion en un fichero de ([\d.]+) líneas ya abierto$")]
async fn complete_big_open_file(w: &mut MoldeWorld, lines: String) {
    let lines: usize = lines.replace('.', "").parse().unwrap();
    let f = "fn f() {\n    let a = 1;\n    let b = a + 2;\n    println!(\"{a} {b}\");\n}\n\n";
    let text = |typed: &str| {
        format!(
            "use std::io;\n\n{}fn main() {{\n    {typed}‸\n}}\n",
            f.repeat(lines / 6)
        )
    };
    w.open("src/grande.rs", "rust", &text("s")).await;
    w.complete().await;
    let mut times = Vec::new();
    for (v, typed) in [
        "sn", "sni", "snip", "snipp", "snippe", "snippet", "snippetx",
    ]
    .iter()
    .enumerate()
    {
        w.change(&text(typed), v as i64 + 2).await;
        let start = Instant::now();
        let items = w.complete().await;
        times.push(start.elapsed().as_secs_f64() * 1000.0);
        assert_eq!(items.len(), 500);
    }
    times.sort_by(f64::total_cmp);
    w.scratch = json!({ "median": times[times.len() / 2], "times": times });
}

#[then(regex = r"^molde responde en menos de (\d+) ms$")]
async fn responds_within(w: &mut MoldeWorld, ms: f64) {
    let median = w.scratch["median"].as_f64().unwrap();
    assert!(
        median < ms,
        "mediana {median:.1} ms: {}",
        w.scratch["times"]
    );
}
