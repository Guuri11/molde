//! 01_completado.feature: capas, lenguajes, contexto e imports.

use crate::world::*;
use cucumber::{given, then, when};
use serde_json::{Value, json};

// Capas y lenguajes

#[given(regex = r#"^un snippet "(.+)" en "(.+)"$"#)]
async fn snippet_in(w: &mut MoldeWorld, key: String, file: String) {
    w.add_snippet(
        &file,
        &key,
        snip(&format!("{key} usuario"), "capa de usuario$0"),
    );
}

#[given(regex = r#"^otro snippet "(.+)" en "(.+)" del proyecto$"#)]
async fn other_snippet_in(w: &mut MoldeWorld, key: String, file: String) {
    w.add_snippet(
        &file,
        &key,
        snip(&format!("{key} proyecto"), "capa de proyecto$0"),
    );
}

#[when(
    regex = r#"^pido completion en un fichero (Rust|Python|TypeScript|"\.tsx")(?: del proyecto)?$"#
)]
async fn complete_in(w: &mut MoldeWorld, lang: String) {
    w.snippet_lang = language_id(&lang).into();
    w.complete().await;
}

#[then("solo aparece la versión del proyecto")]
async fn only_project(w: &mut MoldeWorld) {
    let found: Vec<&Value> = w
        .last()
        .iter()
        .filter(|i| key_of(i) == "test-basico")
        .collect();
    assert_eq!(found.len(), 1, "{:?}", keys(w.last()));
    assert_eq!(found[0]["textEdit"]["newText"], "capa de proyecto$0");
}

#[given(regex = r"^los ficheros de snippets (.+)$")]
async fn snippet_files(w: &mut MoldeWorld, files: String) {
    let files = quoted(&files);
    for f in &files {
        let stem = f.trim_end_matches(".json");
        w.add_snippet(
            &format!(".molde/snippets/{f}"),
            &format!("de-{stem}"),
            snip(&format!("de {stem}"), stem),
        );
    }
    w.scratch = json!(files);
}

#[then(regex = r"^aparecen los snippets de (.+)$")]
async fn appear_from(w: &mut MoldeWorld, files: String) {
    let files: Vec<String> = if files == "los dos ficheros" {
        serde_json::from_value(w.scratch.clone()).unwrap()
    } else {
        quoted(&files)
    };
    for f in files {
        let key = format!("de-{}", f.trim_end_matches(".json"));
        assert!(
            keys(w.last()).contains(&key.as_str()),
            "falta {key}: {:?}",
            keys(w.last())
        );
    }
}

#[then(regex = r"^no aparecen los de (.+)$")]
async fn not_appear_from(w: &mut MoldeWorld, files: String) {
    for f in quoted(&files) {
        let key = format!("de-{}", f.trim_end_matches(".json"));
        assert!(!keys(w.last()).contains(&key.as_str()), "sobra {key}");
    }
}

#[given(regex = r#"^un snippet con prefijo "(.+)"$"#)]
async fn snippet_with_prefix(w: &mut MoldeWorld, prefix: String) {
    w.add_snippet(
        ".molde/snippets/rust.json",
        "con-prefijo",
        snip(&prefix, "handler$0"),
    );
}

#[when(regex = r#"^pido completion tras escribir "(.+)"$"#)]
async fn complete_after_typing(w: &mut MoldeWorld, typed: String) {
    w.open(
        "src/main.rs",
        "rust",
        &format!("fn main() {{}}\n\n{typed}{CURSOR}\n"),
    )
    .await;
    w.scratch = json!(typed);
    w.complete().await;
}

#[then("molde devuelve el item con el prefijo completo como texto de filtrado")]
async fn full_prefix_filter(w: &mut MoldeWorld) {
    let item = w.item("con-prefijo");
    assert_eq!(item["filterText"], "axum handler get by id");
    assert_eq!(item["label"], "axum handler get by id");
}

#[then(regex = r#"^el editor lo encuentra con "(.+)" gracias a su propio filtrado fuzzy$"#)]
async fn editor_fuzzy(w: &mut MoldeWorld, typed: String) {
    let item = w.item("con-prefijo");
    let filter = item["filterText"].as_str().unwrap();
    // Subsecuencia, como cualquier filtrado fuzzy.
    let mut rest = filter.chars();
    assert!(
        typed.chars().all(|c| rest.any(|f| f == c)),
        "{typed} no encaja en {filter}"
    );
    // Lo que se sustituye es justo lo escrito.
    let doc = w.doc.clone().unwrap();
    let start = &item["textEdit"]["range"]["start"];
    assert_eq!(start["character"], 0);
    assert_eq!(
        doc.text
            .lines()
            .nth(start["line"].as_u64().unwrap() as usize)
            .unwrap(),
        typed
    );
}

#[given("que rust-analyzer y molde están activos en el mismo buffer")]
async fn alongside_ra(w: &mut MoldeWorld) {
    w.add_snippet(".molde/snippets/rust.json", "uno", snip("uno", "uno$0"));
}

#[when("pido completion")]
async fn complete(w: &mut MoldeWorld) {
    w.complete().await;
}

#[then(regex = r#"^los items de molde son de tipo Snippet y muestran "(.+)" como origen$"#)]
async fn snippet_kind(w: &mut MoldeWorld, origin: String) {
    assert!(!w.last().is_empty());
    for item in w.last() {
        assert_eq!(item["kind"], 15, "CompletionItemKind.Snippet");
        assert_eq!(item["labelDetails"]["description"], origin.as_str());
        assert_eq!(item["insertTextFormat"], 2);
    }
}

#[then(regex = r#"^su prioridad de ordenación se puede ajustar en "(.+)"$"#)]
async fn sort_priority(w: &mut MoldeWorld, config: String) {
    let before = w.item("uno")["sortText"].as_str().unwrap().to_string();
    assert!(before.starts_with("0050"), "{before}");
    w.write(&config, "sort_priority = 900\n");
    w.complete().await;
    let after = w.item("uno")["sortText"].as_str().unwrap().to_string();
    assert!(after.starts_with("0900") && after > before, "{after}");
}

// Contexto

fn context_fixture(lang: &str, position: &str) -> &'static str {
    match (lang, position) {
        ("rust", "en el nivel superior del fichero") => {
            "use std::io;\n\nfn main() {\n    let x = 1;\n}\n\nahg‸\n"
        }
        ("rust", "dentro del cuerpo de una función") => {
            "fn main() {\n    let x = 1;\n    ahg‸\n}\n"
        }
        ("typescript", "dentro del cuerpo de una clase") => {
            "class Invoice {\n  total = 0;\n  ahg‸\n}\n"
        }
        ("typescript", "en el nivel superior del fichero") => "const a = 1;\nahg‸\n",
        ("python", "dentro de una función") => "def main():\n    x = 1\n    ahg‸\n",
        ("python", "en el nivel superior del módulo") => "x = 1\nahg‸\n",
        ("python", "dentro de una clase") => "class A:\n    x = 1\n    ahg‸\n",
        other => panic!("sin fixture para {other:?}"),
    }
}

#[given(regex = r#"^un snippet de (\w+) con context "(.+)"$"#)]
async fn snippet_with_context(w: &mut MoldeWorld, lang: String, context: String) {
    let lang = language_id(&lang).to_string();
    w.add_snippet(
        &w.project_snippets(&lang),
        "con-context",
        json!({ "prefix": "ctx", "body": ["ctx$0"], "context": [context] }),
    );
    w.snippet_lang = lang;
}

#[given(regex = r#"^un snippet de (\w+) sin campo "context"$"#)]
async fn snippet_without_context(w: &mut MoldeWorld, lang: String) {
    let lang = language_id(&lang).to_string();
    w.add_snippet(
        &w.project_snippets(&lang),
        "con-context",
        snip("libre", "libre$0"),
    );
    w.snippet_lang = lang;
}

#[when(regex = r"^pido completion con el cursor (.+)$")]
async fn complete_at(w: &mut MoldeWorld, position: String) {
    let lang = w.snippet_lang.clone();
    w.open(
        &format!("src/ctx.{}", extension(&lang)),
        &lang,
        context_fixture(&lang, &position),
    )
    .await;
    w.complete().await;
}

#[when(regex = r"^pido completion en cualquier punto de un fichero (\w+)$")]
async fn complete_anywhere(w: &mut MoldeWorld, lang: String) {
    let lang = language_id(&lang);
    for (i, position) in [
        "en el nivel superior del módulo",
        "dentro de una función",
        "dentro de una clase",
    ]
    .iter()
    .enumerate()
    {
        let rel = format!("src/punto{i}.{}", extension(lang));
        w.open(&rel, lang, context_fixture(lang, position)).await;
        w.complete().await;
    }
}

#[then(regex = r"^el snippet (aparece|no aparece)$")]
async fn snippet_appears(w: &mut MoldeWorld, result: String) {
    for items in &w.results {
        assert_eq!(
            keys(items).contains(&"con-context"),
            result == "aparece",
            "{:?}",
            keys(items)
        );
    }
}

#[given("un fichero Go, lenguaje para el que molde no tiene gramática")]
async fn go_file(w: &mut MoldeWorld) {
    w.pending = Some((
        "main.go".into(),
        "go".into(),
        "package main\n\nfunc main() {\n\tahg‸\n}\n".into(),
    ));
}

#[given(regex = r#"^un fichero de snippets "(\w+)\.json"$"#)]
async fn go_snippets(w: &mut MoldeWorld, lang: String) {
    let file = w.project_snippets(&lang);
    w.add_snippet(
        &file,
        "go-en-metodo",
        json!({ "prefix": "metodo", "body": ["m$0"], "context": ["method_declaration"] }),
    );
    w.add_snippet(
        &file,
        "go-con-import",
        json!({ "prefix": "fmt", "body": ["fmt.Println($1)$0"], "requires": ["import \"fmt\""] }),
    );
}

#[then(regex = r#"^aparecen todos los snippets de "(.+)" sin filtrar por contexto$"#)]
async fn all_go(w: &mut MoldeWorld, _file: String) {
    assert_eq!(keys(w.last()), ["go-en-metodo", "go-con-import"]);
}

#[then(regex = r#"^los que declaran "requires" se insertan sin añadir imports$"#)]
async fn go_no_imports(w: &mut MoldeWorld) {
    assert!(w.item("go-con-import")["additionalTextEdits"].is_null());
}

#[then("molde avisa una sola vez en el log de que Go no tiene soporte de contexto ni de imports")]
async fn go_warned_once(w: &mut MoldeWorld) {
    w.complete().await;
    w.complete().await;
    let logs = w.lsp().await.messages("window/logMessage").await;
    let go: Vec<&Value> = logs
        .iter()
        .filter(|m| {
            m["message"]
                .as_str()
                .unwrap_or_default()
                .contains("go no tiene gramática")
        })
        .collect();
    assert_eq!(go.len(), 1, "{logs:?}");
}

#[given("la gramática de Treesitter de Go añadida como dependencia del crate de sintaxis")]
async fn go_grammar(w: &mut MoldeWorld) {
    // La gramática de Go es una dependencia de estos tests; se registra con Treesitter::with_language.
    w.go_queries = Some((String::new(), String::new()));
}

#[given(regex = r#"^sus consultas de contenedores y de imports en "(.+)"$"#)]
async fn go_queries(w: &mut MoldeWorld, dir: String) {
    w.write(&format!("{dir}/containers.scm"), "(source_file) @container\n(function_declaration) @container\n(method_declaration) @container\n");
    w.write(
        &format!("{dir}/imports.scm"),
        "(import_declaration) @import\n(package_clause) @header\n",
    );
    w.go_queries = Some((
        w.read(&format!("{dir}/containers.scm")),
        w.read(&format!("{dir}/imports.scm")),
    ));
}

#[when("recompilo molde")]
async fn rebuild(w: &mut MoldeWorld) {
    w.lsp = None;
    w.add_snippet(".molde/snippets/go.json", "go-en-funcion", json!({ "prefix": "f", "body": ["fmt.Println($1)$0"], "context": ["function_declaration"], "requires": ["import \"fmt\""] }));
}

#[then("Go tiene contexto e imports")]
async fn go_works(w: &mut MoldeWorld) {
    w.open("main.go", "go", "package main\n\nfunc main() {\n\tf‸\n}\n")
        .await;
    let item = w
        .complete()
        .await
        .into_iter()
        .find(|i| key_of(i) == "go-en-funcion")
        .expect("aparece dentro de la función");
    let applied = crate::lsp_client::apply(&w.doc.as_ref().unwrap().text, &item, true);
    assert!(
        applied.starts_with("package main\nimport \"fmt\"\n"),
        "{applied}"
    );
    w.open("top.go", "go", "package main\n\nf‸\n").await;
    assert!(
        !keys(&w.complete().await).contains(&"go-en-funcion"),
        "no aparece en el nivel superior"
    );
}

#[then("el crate del núcleo no ha cambiado")]
async fn core_untouched(_w: &mut MoldeWorld) {
    let core = concat!(env!("CARGO_MANIFEST_DIR"), "/../molde-core");
    let manifest = std::fs::read_to_string(format!("{core}/Cargo.toml")).unwrap();
    assert!(
        !manifest.contains("tree-sitter"),
        "molde-core no depende de Treesitter"
    );
    let mut stack = vec![std::path::PathBuf::from(format!("{core}/src"))];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                let src = std::fs::read_to_string(&p).unwrap();
                let src: String = src
                    .lines()
                    .filter(|l| !l.trim_start().starts_with("//"))
                    .collect();
                for word in ["tree_sitter", "\"go\"", "\"python\"", "\"rust\""] {
                    assert!(!src.contains(word), "{} menciona {word}", p.display());
                }
            }
        }
    }
}

// Imports

#[given(regex = r#"^un snippet (?:de (\w+) )?que requiere (['"].+['"])$"#)]
async fn snippet_requires(w: &mut MoldeWorld, lang: String, require: String) {
    let lang = if lang.is_empty() {
        w.pending
            .as_ref()
            .map(|(_, l, _)| l.clone())
            .unwrap_or_else(|| w.snippet_lang.clone())
    } else {
        language_id(&lang).to_string()
    };
    let require = require[1..require.len() - 1].to_string();
    w.add_snippet(
        &w.project_snippets(&lang),
        "req",
        json!({ "prefix": "req", "body": ["usa(${1:x})$0"], "requires": [require] }),
    );
    w.snippet_lang = lang;
}

#[given(regex = r"^un fichero (\w+) sin ese import$")]
async fn file_without_import(w: &mut MoldeWorld, lang: String) {
    let lang = language_id(&lang);
    w.pending = Some((
        format!("src/a.{}", extension(lang)),
        lang.into(),
        "import { a } from \"a\";\n\nconst x = 1;\nreq‸\n".into(),
    ));
}

#[given(regex = r#"^un fichero (\w+) que contiene "(.+)"$"#)]
async fn file_containing(w: &mut MoldeWorld, lang: String, content: String) {
    let lang = language_id(&lang);
    w.pending = Some((
        format!("src/a.{}", extension(lang)),
        lang.into(),
        format!("{content}\n\nreq‸\n"),
    ));
}

#[when("acepto el snippet")]
async fn accept(w: &mut MoldeWorld) {
    let key = w.last_key.clone();
    w.accept(&key).await;
}

#[then("el item trae una edición adicional que inserta el import junto al resto de imports")]
async fn import_next_to_others(w: &mut MoldeWorld) {
    let item = w.accepted.clone().unwrap();
    assert_eq!(
        item["additionalTextEdits"].as_array().map(Vec::len),
        Some(1)
    );
    let lines = w.applied_lines();
    let other = lines
        .iter()
        .position(|l| l == "import { a } from \"a\";")
        .unwrap();
    assert_eq!(lines[other + 1], "import { z } from \"zod\"", "{lines:?}");
}

#[then("el cursor queda en el primer placeholder del snippet")]
async fn cursor_first_placeholder(w: &mut MoldeWorld) {
    let item = w.accepted.clone().unwrap();
    let doc = w.doc.clone().unwrap();
    assert_eq!(item["insertTextFormat"], 2);
    assert!(
        item["textEdit"]["newText"]
            .as_str()
            .unwrap()
            .contains("${1:")
    );
    // La edición principal es la del cursor: el import va aparte, más arriba.
    assert_eq!(
        item["textEdit"]["range"]["end"],
        crate::lsp_client::pos(&doc.text, doc.cursor, true)
    );
}

#[then("no se añade ninguna línea de import")]
async fn no_import_added(w: &mut MoldeWorld) {
    let item = w.accepted.clone().unwrap();
    assert!(
        item["additionalTextEdits"].is_null(),
        "{}",
        item["additionalTextEdits"]
    );
}

#[then(regex = r#"^solo se añade "(.+)"$"#)]
async fn only_added(w: &mut MoldeWorld, line: String) {
    let edits = w.accepted.clone().unwrap()["additionalTextEdits"].clone();
    assert_eq!(edits.as_array().map(Vec::len), Some(1), "{edits}");
    assert_eq!(edits[0]["newText"].as_str().unwrap().trim(), line);
}

#[then(regex = r#"^se añade "(.+)" en una línea nueva$"#)]
async fn added_on_new_line(w: &mut MoldeWorld, line: String) {
    let edits = w.accepted.clone().unwrap()["additionalTextEdits"].clone();
    assert_eq!(edits[0]["newText"], format!("{line}\n"));
    assert_eq!(edits[0]["range"]["start"]["character"], 0);
    assert!(w.applied_lines().contains(&line));
}

#[then("fusionar las dos líneas queda en manos del organizador de imports del proyecto")]
async fn not_merged(w: &mut MoldeWorld) {
    let lines = w.applied_lines();
    assert_eq!(lines[0], "from fastapi import APIRouter");
    assert_eq!(lines[1], "from fastapi import HTTPException");
}
