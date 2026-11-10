//! 03_uso_y_poda.feature

use crate::lsp_client::path_of;
use crate::world::*;
use cucumber::{given, then, when};
use molde_core::{UsageEvent, UsageStore};
use serde_json::json;

// Registro de usos

#[when(regex = r#"^acepto el snippet "(.+)"$"#)]
async fn accept_and_record(w: &mut MoldeWorld, key: String) {
    w.add_snippet(".molde/snippets/rust.json", &key, axum_get_by_id());
    w.open(
        "src/features/invoice/handlers.rs",
        "rust",
        "use axum::Json;\n\nahg‸\n",
    )
    .await;
    let item = w.accept(&key).await;
    // Lo que hace el editor al aceptar: ejecutar el comando que acompaña al item.
    let command = item["command"].clone();
    w.lsp()
        .await
        .exec(
            command["command"].as_str().unwrap(),
            command["arguments"].clone(),
        )
        .await;
}

#[then(regex = r#"^el editor ejecuta el comando "(.+)" que acompaña al item$"#)]
async fn item_command(w: &mut MoldeWorld, command: String) {
    assert_eq!(
        w.accepted.as_ref().unwrap()["command"]["command"],
        command.as_str()
    );
}

#[then("molde guarda la clave del snippet, la raíz del proyecto, el lenguaje y la fecha")]
async fn stored_fields(w: &mut MoldeWorld) {
    let conn = rusqlite::Connection::open(w.paths().usage_db).unwrap();
    let rows: Vec<(String, String, String, i64)> = conn
        .prepare("SELECT key, project_root, language, used_at FROM uses")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(rows.len(), 1, "{rows:?}");
    let (key, root, language, at) = &rows[0];
    assert_eq!(
        (key.as_str(), root.as_str(), language.as_str()),
        ("axum-get-by-id", w.project.to_str().unwrap(), "rust")
    );
    assert!((at - w.now).abs() < 60, "fecha {at}, esperaba {}", w.now);
}

#[then(regex = r#"^lo guarda en "~/(.+)", fuera del repositorio$"#)]
async fn stored_outside(w: &mut MoldeWorld, rel: String) {
    let db = w.paths().usage_db;
    assert_eq!(db, w.home.join(rel));
    assert!(db.exists() && !db.starts_with(&w.project));
}

#[given(regex = r#"^el snippet "(.+)" con (\d+) usos registrados$"#)]
async fn with_uses(w: &mut MoldeWorld, key: String, n: usize) {
    w.add_snippet(".molde/snippets/rust.json", &key, axum_get_by_id());
    let (id, root) = (w.id("rust", &key), w.project.display().to_string());
    for _ in 0..n {
        w.engine().record_use(id.clone(), root.clone()).unwrap();
    }
}

#[when("cambio su prefijo y su descripción sin tocar su clave")]
async fn change_prefix(w: &mut MoldeWorld) {
    let mut s = axum_get_by_id();
    s["prefix"] = json!("handler por id");
    s["description"] = json!("Otra descripción");
    let key = w.last_key.clone();
    w.add_snippet(".molde/snippets/rust.json", &key, s);
}

fn stats_row(w: &mut MoldeWorld, key: &str) -> molde_core::StatsRow {
    let project = w.project.clone();
    let (rows, _) = w.engine().stats(Some(&project)).unwrap();
    rows.into_iter()
        .find(|r| r.id.key == key)
        .unwrap_or_else(|| panic!("{key} no está en stats"))
}

#[then(regex = r"^sigue teniendo (\d+) usos$")]
async fn still_has(w: &mut MoldeWorld, n: u64) {
    let key = w.last_key.clone();
    assert_eq!(stats_row(w, &key).uses, n);
}

#[given("un snippet que molde no había visto nunca")]
async fn unseen(w: &mut MoldeWorld) {
    w.add_snippet(
        ".molde/snippets/rust.json",
        "nuevo",
        snip("nuevo", "nuevo$0"),
    );
}

#[when("lo carga por primera vez")]
async fn first_load(w: &mut MoldeWorld) {
    w.scratch = json!(stats_row(w, "nuevo").first_seen);
}

#[then("guarda esa fecha como fecha de alta del snippet")]
async fn first_seen_kept(w: &mut MoldeWorld) {
    assert_eq!(w.scratch, json!(w.now));
    // Diez días después sigue teniendo la misma fecha de alta.
    let first = w.now;
    w.now += 10 * DAY;
    w.engine = None;
    assert_eq!(stats_row(w, "nuevo").first_seen, Some(first));
}

// Candidatos a borrar

fn register(w: &mut MoldeWorld, key: &str, days_ago: i64) {
    w.add_snippet(".molde/snippets/rust.json", key, snip(key, "x$0"));
    let id = w.id("rust", key);
    w.usage().register(&[id], w.now - days_ago * DAY).unwrap();
}

fn used(w: &mut MoldeWorld, key: &str, days_ago: i64) {
    let event = UsageEvent {
        id: w.id("rust", key),
        project_root: w.project.display().to_string(),
        at: w.now - days_ago * DAY,
    };
    w.usage().record(&event).unwrap();
}

#[given(regex = r"^un snippet dado de alta hace (\d+) días$")]
async fn registered(w: &mut MoldeWorld, days: i64) {
    register(w, "s", days);
}

#[given(regex = r"^cuyo último uso fue (?:hace (\d+) días|nunca)$")]
async fn last_used(w: &mut MoldeWorld, days: String) {
    if let Ok(d) = days.parse() {
        used(w, "s", d);
    }
}

#[given(regex = r"^un snippet dado de alta hace (\d+) días y usado por última vez hace (\d+)$")]
async fn registered_and_used(w: &mut MoldeWorld, days: i64, last: i64) {
    register(w, "s", days);
    used(w, "s", last);
}

#[when(regex = r#"^ejecuto "molde ([^"]+)"$"#)]
async fn run_cli(w: &mut MoldeWorld, args: String) {
    w.run_cli(&args);
}

#[then(regex = r"^el snippet (aparece|no aparece) como candidato a borrar$")]
async fn delete_candidate(w: &mut MoldeWorld, mark: String) {
    let (code, out, _) = &w.cli;
    assert_eq!(*code, 0);
    let row = out
        .lines()
        .find(|l| l.starts_with("| s |"))
        .unwrap_or_else(|| panic!("{out}"));
    assert_eq!(
        row.trim_end_matches(" |").ends_with("sí"),
        mark == "aparece",
        "{row}"
    );
}

#[when(regex = r#"^ejecuto el comando "(molde\.\w+)" desde el editor$"#)]
async fn editor_command(w: &mut MoldeWorld, command: String) {
    w.lsp().await.exec(&command, json!([])).await;
}

#[then("molde escribe el informe en Markdown en un fichero temporal")]
async fn markdown_report(w: &mut MoldeWorld) {
    let shown = w.shown().await;
    let path = path_of(
        shown.last().expect("abre un documento")["uri"]
            .as_str()
            .unwrap(),
    );
    assert!(
        path.starts_with(std::env::temp_dir()) && path.extension().is_some_and(|e| e == "md"),
        "{}",
        path.display()
    );
    assert!(
        std::fs::read_to_string(&path)
            .unwrap()
            .starts_with("# molde stats")
    );
}

#[then("pide al editor que lo abra")]
async fn asks_to_open(w: &mut MoldeWorld) {
    assert_eq!(w.shown().await.len(), 1);
}

// Regla de tres

fn python_function(name: &str, table: &str, limit: u32) -> String {
    format!(
        "def listar_{name}(db, limite={limit}):\n    filas = db.query(\"SELECT * FROM {table}\", limite)\n    resultado = []\n    for fila in filas:\n        resultado.append(fila[\"id\"])\n    return resultado\n"
    )
}

#[given(
    "tres funciones Python en ficheros distintos con la misma estructura y distintos nombres y literales"
)]
async fn three_functions(w: &mut MoldeWorld) {
    for (name, table, limit) in [
        ("facturas", "invoices", 10),
        ("clientes", "customers", 25),
        ("pedidos", "orders", 50),
    ] {
        w.write(
            &format!("app/{name}.py"),
            &format!("import os\n\n\n{}", python_function(name, table, limit)),
        );
    }
}

#[given("ningún snippet del proyecto con esa estructura")]
async fn no_snippet(w: &mut MoldeWorld) {
    assert!(!w.path(".molde/snippets/python.json").exists());
}

fn section(out: &str, n: usize) -> String {
    out.split("\n## ")
        .find(|s| s.starts_with(&format!("{n}. ")))
        .unwrap_or_else(|| panic!("no hay candidato {n}:\n{out}"))
        .to_string()
}

#[then("aparece un candidato con las tres ubicaciones")]
async fn candidate_with_three(w: &mut MoldeWorld) {
    let first = section(&w.cli.1, 1);
    for f in [
        "app/clientes.py:4-9",
        "app/facturas.py:4-9",
        "app/pedidos.py:4-9",
    ] {
        assert!(first.contains(f), "falta {f}:\n{}", w.cli.1);
    }
    assert!(
        !w.cli.1.contains("## 2."),
        "solo un candidato:\n{}",
        w.cli.1
    );
}

#[then("puedo crearlo con la misma generalización que usa la captura")]
async fn save_candidate(w: &mut MoldeWorld) {
    std::fs::create_dir_all(w.path(".molde")).unwrap();
    w.run_cli("candidates --save 1");
    assert_eq!(w.cli.0, 0, "{}", w.cli.2);
    let saved: serde_json::Value =
        serde_json::from_str(&w.read(".molde/snippets/python.json")).unwrap();
    let body = saved["captura-1"]["body"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l.as_str().unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    let selection = python_function("clientes", "customers", 25);
    assert_eq!(
        body,
        w.engine().generalize("python", selection.trim_end(), &[])
    );
    assert!(body.contains("${1:"), "{body}");
}

#[given("patrones repetidos de menos de 5 líneas")]
async fn short_patterns(w: &mut MoldeWorld) {
    for name in ["a", "b", "c"] {
        w.write(
            &format!("app/{name}.py"),
            &format!("def corta_{name}(x):\n    y = x + 1\n    z = y * 2\n    return z\n"),
        );
    }
}

#[given(regex = r#"^ficheros ignorados por "\.gitignore", como (.+)$"#)]
async fn ignored(w: &mut MoldeWorld, dirs: String) {
    let dirs = quoted(&dirs);
    w.write(
        ".gitignore",
        &dirs.iter().map(|d| format!("{d}/\n")).collect::<String>(),
    );
    for (i, d) in dirs.iter().enumerate() {
        w.write(
            &format!("{d}/generado.py"),
            &python_function(&format!("g{i}"), "t", i as u32),
        );
    }
}

#[then("no aparecen en el informe")]
async fn not_in_report(w: &mut MoldeWorld) {
    let out = &w.cli.1;
    assert!(out.contains("Sin candidatos"), "{out}");
    for d in ["target", "node_modules", "dist"] {
        assert!(!out.contains(d), "{out}");
    }
}

#[given(regex = r#"^tres funciones con la misma estructura que el cuerpo del snippet "(.+)"$"#)]
async fn covered_functions(w: &mut MoldeWorld, key: String) {
    w.add_snippet(".molde/snippets/rust.json", &key, axum_get_by_id());
    for (snake, pascal) in [
        ("invoice", "Invoice"),
        ("customer", "Customer"),
        ("order", "Order"),
    ] {
        let code = format!(
            "pub async fn get_{snake}(\n    State(svc): State<Arc<{pascal}Service>>,\n    Path(id): Path<Uuid>,\n) -> Result<Json<{pascal}Dto>, ApiError> {{\n    let {snake} = svc.find(id).await?;\n    Ok(Json({snake}.into()))\n}}\n"
        );
        w.write(&format!("src/{snake}.rs"), &code);
    }
}

#[then("ese patrón no aparece como candidato")]
async fn covered_not_candidate(w: &mut MoldeWorld) {
    assert!(w.cli.1.contains("Sin candidatos"), "{}", w.cli.1);
}

#[when("edito código con patrones repetidos")]
async fn edit_repeated(w: &mut MoldeWorld) {
    three_functions(w).await;
    let text = w.read("app/facturas.py");
    w.open("app/facturas.py", "python", &text).await;
    w.change(&format!("{text}\n‸"), 2).await;
    w.complete().await;
}

#[then("molde no publica diagnósticos sobre ellos")]
async fn no_diagnostics(w: &mut MoldeWorld) {
    assert!(
        w.lsp()
            .await
            .messages("textDocument/publishDiagnostics")
            .await
            .is_empty()
    );
}

#[then(
    regex = r#"^solo los muestra cuando ejecuto "molde candidates" o el comando "molde\.candidates"$"#
)]
async fn shown_on_demand(w: &mut MoldeWorld) {
    w.lsp().await.exec("molde.candidates", json!([])).await;
    let report = path_of(w.shown().await[0]["uri"].as_str().unwrap());
    let md = std::fs::read_to_string(report).unwrap();
    w.run_cli("candidates");
    for out in [md, w.cli.1.clone()] {
        assert!(
            out.contains("app/facturas.py:4-9") && out.contains("app/pedidos.py:4-9"),
            "{out}"
        );
    }
}

#[given(regex = r"^(\d+) patrones repetidos sin snippet, cada uno un número de veces distinto$")]
async fn many_patterns(w: &mut MoldeWorld, n: usize) {
    // El patrón p tiene 5 + p sentencias y se repite 3 + p veces: estructuras y recuentos distintos.
    for p in 0..n {
        let functions: Vec<String> = (0..3 + p)
            .map(|r| {
                let body: String = (0..5 + p)
                    .map(|k| format!("    v{k} = x + {k}\n"))
                    .collect();
                format!("def f_{p}_{r}(x):\n{body}")
            })
            .collect();
        w.write(&format!("app/patron_{p}.py"), &functions.join("\n\n"));
    }
}

/// Apariciones de cada candidato del informe, en orden.
fn counts(out: &str) -> Vec<usize> {
    out.lines()
        .filter_map(|l| l.strip_prefix("## "))
        .map(|h| {
            h.split(", ")
                .nth(1)
                .unwrap()
                .split(' ')
                .next()
                .unwrap()
                .parse()
                .unwrap()
        })
        .collect()
}

#[then(regex = r"^el informe enseña (\d+) candidatos, de más a menos apariciones$")]
async fn shows_top(w: &mut MoldeWorld, n: usize) {
    let expected: Vec<usize> = (0..n).map(|i| 27 - i).collect();
    assert_eq!(counts(&w.cli.1), expected, "{}", w.cli.1);
}

#[then(regex = r"^al final dice que hay (\d+) en total$")]
async fn says_total(w: &mut MoldeWorld, total: usize) {
    let last = w.cli.1.trim_end().lines().last().unwrap();
    assert!(last.contains(&format!("de {total} candidatos")), "{last}");
}
