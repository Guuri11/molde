//! 04_scaffold.feature

use crate::lsp_client::path_of;
use crate::world::*;
use cucumber::gherkin::Step;
use cucumber::{given, then, when};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// La tabla de variantes de la especificación, de la más larga a la más corta.
const VARIANTS: [(&str, &str); 10] = [
    ("sample_widgets", "line_items"),
    ("SampleWidgets", "LineItems"),
    ("sample-widgets", "line-items"),
    ("sample_widget", "line_item"),
    ("SampleWidget", "LineItem"),
    ("sampleWidget", "lineItem"),
    ("sample-widget", "line-item"),
    ("SAMPLE_WIDGET", "LINE_ITEM"),
    ("Sample Widget", "Line Item"),
    ("sample widget", "line item"),
];

/// Lo que debería salir según una tabla de variantes, sustituyendo primero la más larga.
fn oracle(text: &str, table: &[(String, String)]) -> String {
    let mut sorted = table.to_vec();
    sorted.sort_by_key(|(a, _)| std::cmp::Reverse(a.len()));
    let (mut out, mut i) = (String::new(), 0);
    'outer: while i < text.len() {
        for (a, b) in &sorted {
            if text[i..].starts_with(a.as_str()) {
                out.push_str(b);
                i += a.len();
                continue 'outer;
            }
        }
        let c = text[i..].chars().next().unwrap();
        out.push(c);
        i += c.len_utf8();
    }
    out
}

fn files_under(dir: &Path) -> BTreeMap<PathBuf, String> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                if p.file_name().is_some_and(|n| n != "__pycache__") {
                    stack.push(p);
                }
            } else {
                out.insert(
                    p.strip_prefix(dir).unwrap().to_path_buf(),
                    std::fs::read_to_string(&p).unwrap_or_default(),
                );
            }
        }
    }
    out
}

fn squash(s: &str) -> String {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    s.replace(",}", "}").replace(",)", ")").replace(",]", "]")
}

/// Cada fichero de la feature dorada tiene su versión generada, en su ruta sustituida y con el
/// contenido sustituido. Sin espacios ni comas finales, porque el formateador puede volver a partir las líneas.
fn assert_generated(w: &MoldeWorld, source: &str, target: &str, table: &[(String, String)]) {
    let golden = files_under(&w.path(source));
    let generated = files_under(&w.path(target));
    assert_eq!(golden.len(), generated.len(), "{:?}", generated.keys());
    for (rel, content) in &golden {
        let rel = PathBuf::from(oracle(&rel.to_string_lossy(), table));
        let got = generated
            .get(&rel)
            .unwrap_or_else(|| panic!("falta {}: {:?}", rel.display(), generated.keys()));
        assert_eq!(
            squash(got),
            squash(&oracle(content, table)),
            "{}",
            rel.display()
        );
        assert!(
            !got.to_lowercase().contains("sample"),
            "{}: {got}",
            rel.display()
        );
    }
}

fn spec_table() -> Vec<(String, String)> {
    VARIANTS
        .iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}

const ROUTER: &str = r#"use crate::features::*;

pub struct Router(pub Vec<&'static str>);

impl Router {
    pub fn new() -> Self {
        Router(Vec::new())
    }

    pub fn route(mut self, path: &'static str) -> Self {
        self.0.push(path);
        self
    }

    pub fn merge(mut self, other: Router) -> Self {
        self.0.extend(other.0);
        self
    }
}

pub fn app() -> Router {
    Router::new()
        // molde:routes
        .merge(sample_widget::routes())
}
"#;

const GOLDEN_MOD: &str = r#"//! Feature dorada: Sample Widget, o sample widget en minúsculas.
mod sample_widget_dto;

pub use sample_widget_dto::SampleWidgetDto;

use crate::router::Router;

pub const SAMPLE_WIDGET_LIMIT: usize = 10;

pub type SampleWidgets = Vec<SampleWidgetDto>;

pub fn routes() -> Router {
    Router::new().route("/sample-widgets").route("/sample-widget/{id}")
}

pub fn list_sample_widgets() -> SampleWidgets {
    #[allow(non_snake_case)]
    let sampleWidget = SampleWidgetDto::new("Sample Widget");
    vec![sampleWidget; SAMPLE_WIDGET_LIMIT]
}

#[cfg(test)]
mod tests {
    #[test]
    fn lists_sample_widgets() {
        assert_eq!(super::list_sample_widgets().len(), super::SAMPLE_WIDGET_LIMIT);
    }
}
"#;

const GOLDEN_DTO: &str = r#"#[derive(Debug, Clone, PartialEq)]
pub struct SampleWidgetDto {
    pub title: String,
}

impl SampleWidgetDto {
    pub fn new(title: &str) -> Self {
        SampleWidgetDto { title: title.to_string() }
    }
}
"#;

fn rust_golden(w: &MoldeWorld, dir: &str) {
    w.write(
        "Cargo.toml",
        "[package]\nname = \"golden\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
    );
    w.write(".gitignore", "target/\n__pycache__/\n");
    w.write(
        "src/main.rs",
        "mod features;\nmod router;\n\nfn main() {\n    println!(\"{} rutas\", router::app().0.len());\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn has_routes() {\n        assert!(!crate::router::app().0.is_empty());\n    }\n}\n",
    );
    w.write("src/router.rs", ROUTER);
    w.write(
        "src/features/mod.rs",
        "// molde:mods\npub mod sample_widget;\n",
    );
    w.write(&format!("{dir}/mod.rs"), GOLDEN_MOD);
    w.write(&format!("{dir}/sample_widget_dto.rs"), GOLDEN_DTO);
    w.write(
        &format!("{dir}/sample-widgets.md"),
        "# Sample Widgets\n\nDocumentación de los sample widgets.\n",
    );
}

fn ts_golden(w: &MoldeWorld, dir: &str) {
    // Node ejecuta TypeScript quitando los tipos: hace de comprobación sin instalar tsc.
    let check = "for f in src/modules/*/index.ts; do node \"$f\" > /dev/null || exit 1; done";
    let package = json!({ "name": "golden-ts", "private": true, "type": "module", "scripts": { "typecheck": check, "lint": check } });
    w.write(
        "package.json",
        &serde_json::to_string_pretty(&package).unwrap(),
    );
    w.write(
        &format!("{dir}/index.ts"),
        "import { SampleWidgetService, listSampleWidgets } from \"./sample-widget.service.ts\";\n\nexport const SAMPLE_WIDGET_ROUTE = \"/sample-widgets\";\n\nconst sampleWidget = new SampleWidgetService(\"Sample Widget\");\nconsole.log(sampleWidget.describe(), listSampleWidgets().length, SAMPLE_WIDGET_ROUTE);\n",
    );
    w.write(
        &format!("{dir}/sample-widget.service.ts"),
        "// sample_widgets y sample_widget, por si alguien los busca.\nexport type SampleWidgets = SampleWidgetService[];\n\nexport class SampleWidgetService {\n  title: string;\n\n  constructor(title: string) {\n    this.title = title;\n  }\n\n  describe(): string {\n    return `sample widget: ${this.title}`;\n  }\n}\n\nexport function listSampleWidgets(): SampleWidgets {\n  return [new SampleWidgetService(\"sample-widget\")];\n}\n",
    );
}

fn python_golden(w: &MoldeWorld, dir: &str) {
    w.write(
        &format!("{dir}/__init__.py"),
        "\"\"\"Feature dorada: Sample Widget.\"\"\"\nfrom .service import SampleWidgetService, list_sample_widgets\n\nSAMPLE_WIDGET_ROUTE = \"/sample-widgets\"\n",
    );
    w.write(
        &format!("{dir}/service.py"),
        "class SampleWidgetService:\n    def __init__(self, title):\n        self.title = title\n\n    def describe(self):\n        return f\"sample widget: {self.title}\"\n\n\ndef list_sample_widgets():\n    sampleWidget = SampleWidgetService(\"sample-widget\")\n    SampleWidgets = [sampleWidget, SampleWidgetService(\"sample_widget\")]\n    return SampleWidgets\n",
    );
    // Sustituto de ruff para no depender de tenerlo instalado: comprueba la sintaxis.
    w.write(
        "bin/ruff",
        "#!/bin/sh\nexec find app -name '*.py' -exec python3 -m py_compile {} +\n",
    );
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(w.path("bin/ruff"), std::fs::Permissions::from_mode(0o755)).unwrap();
}

#[given(regex = r#"^"(.+)" con:$"#)]
async fn file_with(w: &mut MoldeWorld, file: String, step: &Step) {
    w.write(
        &file,
        step.docstring.as_deref().unwrap().trim_start_matches('\n'),
    );
}

#[given(regex = r#"^la feature dorada "(.+)" compila y sus tests pasan$"#)]
async fn golden_compiles(w: &mut MoldeWorld, dir: String) {
    rust_golden(w, &dir);
    let project = w.project.clone();
    let out = blocking(|| {
        std::process::Command::new("cargo")
            .args(["test", "-q"])
            .current_dir(&project)
            .output()
            .unwrap()
    });
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[then(regex = r#"^se crea "(.+)" a partir de la feature dorada$"#)]
async fn created(w: &mut MoldeWorld, target: String) {
    assert_eq!(w.cli.0, 0, "{}", w.cli.2);
    assert!(w.path(&target).is_dir());
    w.scratch = json!(target);
}

#[then("en nombres de fichero y en contenido se sustituyen estas variantes:")]
async fn variants(w: &mut MoldeWorld, step: &Step) {
    let table: Vec<(String, String)> = step
        .table
        .as_ref()
        .unwrap()
        .rows
        .iter()
        .skip(1)
        .map(|r| (r[0].clone(), r[1].clone()))
        .collect();
    // Que la feature dorada use de verdad cada variante.
    let golden: String = files_under(&w.path("src/features/sample_widget"))
        .into_iter()
        .map(|(p, c)| format!("{}\n{c}", p.display()))
        .collect();
    for (from, _) in &table {
        assert!(
            golden.contains(from.as_str()),
            "la feature dorada no usa {from}"
        );
    }
    assert_generated(
        w,
        "src/features/sample_widget",
        w.scratch.as_str().unwrap(),
        &table,
    );
}

#[then(regex = r#"^"(.+)" pasa a ser "(.+)" y "(.+)" pasa a ser "(.+)"$"#)]
async fn plural(w: &mut MoldeWorld, _a: String, b: String, _c: String, d: String) {
    assert_eq!(w.cli.0, 0, "{}", w.cli.2);
    let generated: String = files_under(&w.path("src/features/person"))
        .into_values()
        .collect();
    assert!(
        generated.contains(&format!("list_{b}")) && generated.contains(&format!("type {d} ")),
        "{generated}"
    );
    assert!(!generated.contains("persons") && !generated.contains("Persons"));
}

#[then(regex = r#"^"(.+)" tiene "(.+)" justo después de "(.+)"$"#)]
async fn line_after_marker(w: &mut MoldeWorld, file: String, line: String, marker: String) {
    let text = w.read(&file);
    let lines: Vec<&str> = text.lines().collect();
    let at = lines
        .iter()
        .position(|l| l.contains(&marker))
        .unwrap_or_else(|| panic!("{file} sin {marker}"));
    assert_eq!(lines[at + 1].trim(), line, "{text}");
}

#[then(regex = r#"^se ejecutan "(.+)" y "(.+)" y el comando termina con código 0$"#)]
async fn post_ran(w: &mut MoldeWorld, a: String, b: String) {
    let (code, _, err) = &w.cli;
    assert_eq!(*code, 0, "{err}");
    assert!(
        err.contains(&format!("ok: {a}")) && err.contains(&format!("ok: {b}")),
        "{err}"
    );
    assert!(w.path("target").is_dir());
}

#[given(regex = r#"^que "(.+)" no contiene el marcador "(.+)"$"#)]
async fn without_marker(w: &mut MoldeWorld, file: String, marker: String) {
    let text: String = w
        .read(&file)
        .lines()
        .filter(|l| !l.contains(&marker))
        .map(|l| format!("{l}\n"))
        .collect();
    w.write(&file, &text);
}

#[then("el comando falla indicando el fichero y el marcador que faltan")]
async fn fails_naming_marker(w: &mut MoldeWorld) {
    let (code, _, err) = &w.cli;
    assert_ne!(*code, 0);
    assert!(
        err.contains("src/router.rs") && err.contains("// molde:routes"),
        "{err}"
    );
}

#[then("no se ha creado ni modificado ningún fichero")]
async fn nothing_changed(w: &mut MoldeWorld) {
    assert!(w.take_snapshot() == w.snapshot, "han cambiado ficheros");
}

#[given(regex = r#"^que ya existe "(.+)"$"#)]
async fn already_exists(w: &mut MoldeWorld, dir: String) {
    w.write(&format!("{dir}/existente.rs"), "// ya estaba\n");
}

#[then("el comando falla sin tocar nada")]
async fn fails_untouched(w: &mut MoldeWorld) {
    assert_ne!(w.cli.0, 0);
    assert!(w.cli.2.contains("ya existe"), "{}", w.cli.2);
    assert!(w.take_snapshot() == w.snapshot, "han cambiado ficheros");
}

#[given(regex = r#"^que "cargo check" falla tras generar la feature$"#)]
async fn check_fails(w: &mut MoldeWorld) {
    let file = ".molde/templates/feature-crud/template.toml";
    let text = w
        .read(file)
        .replace("{snake}::routes()", "{snake}::no_existe()");
    w.write(file, &text);
}

#[then("los ficheros generados se quedan en su sitio para revisarlos")]
async fn generated_kept(w: &mut MoldeWorld) {
    assert!(w.path("src/features/line_item/mod.rs").exists());
    assert!(
        w.cli
            .1
            .lines()
            .any(|l| l == "A src/features/line_item/mod.rs"),
        "{}",
        w.cli.1
    );
}

#[then("molde muestra la salida del comando y termina con un código distinto de 0")]
async fn shows_output(w: &mut MoldeWorld) {
    let (code, _, err) = &w.cli;
    assert_ne!(*code, 0);
    assert!(
        err.contains("cargo check") && err.contains("no_existe"),
        "{err}"
    );
}

#[given(regex = r#"^una plantilla de (\w+) con la feature dorada en "(.+)"$"#)]
async fn template_for(w: &mut MoldeWorld, lang: String, source: String) {
    match language_id(&lang) {
        "rust" => rust_golden(w, &source),
        "typescript" => ts_golden(w, &source),
        _ => python_golden(w, &source),
    }
    w.scratch = json!({ "template": format!("feature-{}", lang.to_lowercase()), "source": source });
}

#[given(regex = r"^los comandos posteriores (.+)$")]
async fn post_commands(w: &mut MoldeWorld, commands: String) {
    let source = w.scratch["source"].as_str().unwrap().to_string();
    let target = source
        .replace("sample_widget", "{snake}")
        .replace("sample-widget", "{kebab}");
    let post: Vec<String> = quoted(&commands);
    let toml = format!(
        "source = \"{source}\"\nsource_name = \"sample widget\"\ntarget = \"{target}\"\npost = {post:?}\n"
    );
    w.write(
        &format!(
            ".molde/templates/{}/template.toml",
            w.scratch["template"].as_str().unwrap()
        ),
        &toml,
    );
    w.scratch["target"] = json!(
        target
            .replace("{snake}", "line_item")
            .replace("{kebab}", "line-item")
    );
}

#[when(regex = r#"^genero una feature "(.+)"$"#)]
async fn generate(w: &mut MoldeWorld, name: String) {
    let template = w.scratch["template"].as_str().unwrap().to_string();
    w.run_cli(&format!("scaffold {template} {name}"));
}

#[then("la feature se crea con las mismas reglas de sustitución")]
async fn same_rules(w: &mut MoldeWorld) {
    assert_eq!(w.cli.0, 0, "{}\n{}", w.cli.1, w.cli.2);
    let (source, target) = (
        w.scratch["source"].as_str().unwrap().to_string(),
        w.scratch["target"].as_str().unwrap().to_string(),
    );
    assert_generated(w, &source, &target, &spec_table());
}

#[then("los comandos posteriores terminan con código 0")]
async fn post_ok(w: &mut MoldeWorld) {
    assert_eq!(w.cli.0, 0, "{}", w.cli.2);
    assert!(
        w.cli.2.lines().filter(|l| l.starts_with("ok: ")).count() == 2,
        "{}",
        w.cli.2
    );
}

#[when(regex = r#"^Claude Code ejecuta "molde ([^"]+)"$"#)]
async fn agent_runs(w: &mut MoldeWorld, args: String) {
    w.run_cli(&args);
}

#[then("la salida lista los ficheros creados y modificados, uno por línea")]
async fn lists_files(w: &mut MoldeWorld) {
    let lines: Vec<&str> = w.cli.1.lines().collect();
    let created = lines.iter().filter(|l| l.starts_with("A ")).count();
    let modified: Vec<&&str> = lines.iter().filter(|l| l.starts_with("M ")).collect();
    assert_eq!(created + modified.len(), lines.len(), "{}", w.cli.1);
    assert_eq!(created, 3);
    assert_eq!(modified, [&"M src/features/mod.rs", &"M src/router.rs"]);
    for l in &lines {
        assert!(w.path(&l[2..]).is_file(), "{l}");
    }
}

#[then("el código de salida indica si ha ido bien")]
async fn exit_code(w: &mut MoldeWorld) {
    assert_eq!(w.cli.0, 0);
    w.run_cli("scaffold feature-crud line-item");
    assert_ne!(w.cli.0, 0, "repetirlo falla porque ya existe");
}

#[then("no hay ninguna pregunta interactiva")]
async fn no_questions(w: &mut MoldeWorld) {
    // molde_cli::run no recibe stdin: no tiene a quién preguntar.
    assert!(
        !w.cli.1.contains('?') && !w.cli.2.contains('?'),
        "{}{}",
        w.cli.1,
        w.cli.2
    );
}

#[then("comprueba que la feature dorada existe y que todos los marcadores están en su sitio")]
async fn check_ok(w: &mut MoldeWorld) {
    assert_eq!(w.cli.0, 0, "{}", w.cli.2);
    let router = w.read("src/router.rs");
    w.write("src/router.rs", &router.replace("// molde:routes", ""));
    w.run_cli("check");
    assert!(
        w.cli.2.contains(
            "plantilla feature-crud: src/router.rs: falta el marcador \"// molde:routes\""
        ),
        "{}",
        w.cli.2
    );
    let mut failures = vec![w.cli.0];
    w.write("src/router.rs", &router);
    std::fs::rename(
        w.path("src/features/sample_widget"),
        w.path("src/features/otra"),
    )
    .unwrap();
    w.run_cli("check");
    assert!(
        w.cli
            .2
            .contains("no existe la feature dorada src/features/sample_widget"),
        "{}",
        w.cli.2
    );
    failures.push(w.cli.0);
    w.scratch = json!(failures);
}

#[then("falla si la plantilla ya no se puede aplicar")]
async fn check_fails_when_broken(w: &mut MoldeWorld) {
    assert_eq!(w.scratch, json!([1, 1]));
}

#[when(
    regex = r#"^ejecuto el comando "molde\.scaffold" con la plantilla "(.+)" y el nombre "(.+)"$"#
)]
async fn editor_scaffold(w: &mut MoldeWorld, template: String, name: String) {
    w.lsp()
        .await
        .exec("molde.scaffold", json!([template, name]))
        .await;
}

#[then("molde genera la feature igual que la CLI")]
async fn like_cli(w: &mut MoldeWorld) {
    let errors = w.lsp().await.messages("window/showMessage").await;
    assert!(errors.is_empty(), "{errors:?}");
    assert_generated(
        w,
        "src/features/sample_widget",
        "src/features/line_item",
        &spec_table(),
    );
    assert!(
        w.read("src/router.rs")
            .contains(".merge(line_item::routes())")
    );
}

#[then("pide al editor que abra el fichero principal de la feature")]
async fn opens_main(w: &mut MoldeWorld) {
    let shown = w.shown().await;
    assert_eq!(
        path_of(shown[0]["uri"].as_str().unwrap()),
        w.path("src/features/line_item/mod.rs")
    );
}
