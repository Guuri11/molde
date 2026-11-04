//! Composición de adaptadores y subcomandos de la CLI. Nunca pregunta nada: pensado también para agentes.

use molde_core::{Engine, Ports, ScaffoldError, candidates_markdown, stats_markdown};
use molde_store::{JsonSnippets, SqliteUsage, StdFs, StdShell, SystemClock};
use molde_syntax::Treesitter;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const USAGE: &str = "uso: molde <comando>

  lsp                                   servidor LSP por stdio
  check                                 valida snippets, configuración y plantillas
  stats                                 usos de cada snippet y candidatos a borrar
  candidates [--save N]                 patrones repetidos sin snippet; --save guarda el N
  scaffold <plantilla> <nombre> [plural=<plural>]
                                        genera una feature desde la feature dorada";

pub struct Paths {
    pub user_dir: PathBuf,
    pub usage_db: PathBuf,
}

impl Paths {
    /// `~/.config/molde/snippets` y `~/.local/share/molde/usage.sqlite`, respetando XDG.
    pub fn from_vars(var: impl Fn(&str) -> Option<String>) -> Paths {
        let home = PathBuf::from(var("HOME").unwrap_or_default());
        let dir = |xdg: &str, default: &str| {
            var(xdg)
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(default))
        };
        Paths {
            user_dir: dir("XDG_CONFIG_HOME", ".config").join("molde/snippets"),
            usage_db: dir("XDG_DATA_HOME", ".local/share").join("molde/usage.sqlite"),
        }
    }
}

pub fn engine(paths: &Paths) -> Result<Engine, String> {
    let ports = Ports {
        snippets: Box::new(JsonSnippets),
        usage: Box::new(SqliteUsage::open(&paths.usage_db)?),
        syntax: Box::new(Treesitter::new()),
        clock: Box::new(SystemClock),
        fs: Box::new(StdFs),
        shell: Box::new(StdShell),
    };
    Ok(Engine::new(ports, paths.user_dir.clone()))
}

/// Ejecuta un subcomando y devuelve el código de salida.
pub fn run(
    args: &[String],
    cwd: &Path,
    engine: &mut Engine,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let root = engine.find_project(cwd);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args.as_slice() {
        ["check"] => check(engine, root.as_deref(), out, err),
        ["stats"] => engine.stats(root.as_deref()).map(|(rows, _)| {
            let _ = write!(out, "{}", stats_markdown(&rows));
            0
        }),
        ["candidates", rest @ ..] => candidates(engine, root.as_deref().unwrap_or(cwd), rest, out),
        ["scaffold", template, name, rest @ ..] => match root.as_deref() {
            Some(root) => Ok(scaffold(engine, root, template, name, rest, out, err)),
            None => {
                Err("no hay carpeta .molde en este directorio ni en los superiores".to_string())
            }
        },
        _ => {
            let _ = writeln!(err, "{USAGE}");
            return 2;
        }
    };
    result.unwrap_or_else(|e| {
        let _ = writeln!(err, "molde: {e}");
        1
    })
}

fn check(
    engine: &mut Engine,
    root: Option<&Path>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<i32, String> {
    let errors = engine.check(root);
    for e in &errors {
        let _ = writeln!(err, "{e}");
    }
    if errors.is_empty() {
        let _ = writeln!(out, "molde check: todo correcto");
        Ok(0)
    } else {
        let _ = writeln!(err, "molde check: {} errores", errors.len());
        Ok(1)
    }
}

fn candidates(
    engine: &mut Engine,
    root: &Path,
    rest: &[&str],
    out: &mut dyn Write,
) -> Result<i32, String> {
    let (found, _) = engine.candidates(root);
    match rest {
        [] => {
            let _ = write!(out, "{}", candidates_markdown(&found));
            Ok(0)
        }
        ["--save", n] => {
            let n: usize = n
                .parse()
                .map_err(|_| format!("--save espera un número, no {n}"))?;
            let c = found
                .get(n.wrapping_sub(1))
                .ok_or_else(|| format!("no hay candidato {n}"))?;
            let saved = engine.save_candidate(root, c)?;
            let file = saved.file.strip_prefix(root).unwrap_or(&saved.file);
            let _ = writeln!(
                out,
                "{} añadido en {}:{}",
                saved.snippet.key,
                file.display(),
                saved.line + 1
            );
            Ok(0)
        }
        _ => Err(format!("argumentos no válidos: {}", rest.join(" "))),
    }
}

fn scaffold(
    engine: &mut Engine,
    root: &Path,
    template: &str,
    name: &str,
    rest: &[&str],
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let mut plural: Option<&str> = None;
    for opt in rest {
        match opt.strip_prefix("plural=") {
            Some(p) => plural = Some(p),
            None => {
                let _ = writeln!(err, "molde: opción desconocida {opt}\n{USAGE}");
                return 2;
            }
        }
    }
    let list = |out: &mut dyn Write, created: &[PathBuf], modified: &[PathBuf]| {
        for p in created {
            let _ = writeln!(out, "A {}", p.display());
        }
        for p in modified {
            let _ = writeln!(out, "M {}", p.display());
        }
    };
    match engine.scaffold(root, template, name, plural) {
        Ok(done) => {
            list(out, &done.created, &done.modified);
            for c in &done.ran {
                let _ = writeln!(err, "ok: {c}");
            }
            0
        }
        Err(ScaffoldError::Invalid(errors)) => {
            for e in errors {
                let _ = writeln!(err, "molde scaffold: {e}");
            }
            let _ = writeln!(err, "molde scaffold: no se ha creado ni modificado nada");
            1
        }
        Err(ScaffoldError::Post {
            done,
            command,
            output,
        }) => {
            list(out, &done.created, &done.modified);
            let _ = writeln!(
                err,
                "molde scaffold: falló \"{command}\"; los ficheros generados se quedan para revisarlos\n{output}"
            );
            1
        }
    }
}
