use super::*;
use crate::domain::names;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Scaffolded {
    /// Relativos a la raíz del proyecto.
    pub created: Vec<PathBuf>,
    pub modified: Vec<PathBuf>,
    pub main: Option<PathBuf>,
    pub ran: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScaffoldError {
    /// No se ha escrito nada.
    Invalid(Vec<String>),
    /// Los ficheros se quedan escritos para revisarlos.
    Post {
        done: Scaffolded,
        command: String,
        output: String,
    },
}

struct Plan {
    files: Vec<(PathBuf, Vec<u8>)>,
    injects: Vec<(PathBuf, String)>,
    post: Vec<String>,
    main: Option<PathBuf>,
}

impl Engine {
    /// Genera una feature a partir de la feature dorada de la plantilla.
    /// Todo se valida antes de escribir; si falla un comando posterior, lo generado se queda.
    pub fn scaffold(
        &mut self,
        root: &Path,
        template: &str,
        name: &str,
        plural: Option<&str>,
    ) -> Result<Scaffolded, ScaffoldError> {
        let plan = self
            .plan(root, template, name, plural)
            .map_err(ScaffoldError::Invalid)?;
        let mut done = Scaffolded {
            main: plan.main,
            ..Default::default()
        };
        let write = |path: &Path, content: &[u8]| {
            self.ports
                .fs
                .write(&root.join(path), content)
                .map_err(|e| format!("{}: {e}", path.display()))
        };
        for (path, content) in &plan.files {
            write(path, content).map_err(|e| ScaffoldError::Invalid(vec![e]))?;
            done.created.push(path.clone());
        }
        for (path, content) in &plan.injects {
            write(path, content.as_bytes()).map_err(|e| ScaffoldError::Invalid(vec![e]))?;
            done.modified.push(path.clone());
        }
        for command in plan.post {
            let (ok, output) = self.ports.shell.run(&command, root);
            if !ok {
                return Err(ScaffoldError::Post {
                    done,
                    command,
                    output,
                });
            }
            done.ran.push(command);
        }
        Ok(done)
    }

    /// Errores que impedirían aplicar la plantilla.
    pub fn check_template(&self, root: &Path, template: &str) -> Vec<String> {
        // Un nombre que no existe en el proyecto: solo interesa si la plantilla se puede aplicar.
        self.plan(root, template, "molde check probe", None)
            .err()
            .unwrap_or_default()
    }

    fn plan(
        &self,
        root: &Path,
        template: &str,
        name: &str,
        plural: Option<&str>,
    ) -> Result<Plan, Vec<String>> {
        let fs = &self.ports.fs;
        let tpath = Path::new(".molde/templates")
            .join(template)
            .join("template.toml");
        let text = fs
            .read(&root.join(&tpath))
            .map_err(|_| vec![format!("no existe la plantilla {}", tpath.display())])?;
        let t = Template::parse(&String::from_utf8_lossy(&text))
            .map_err(|e| vec![format!("{}: {e}", tpath.display())])?;

        let mut errors = Vec::new();
        let from = names::words(&t.source_name);
        let to = names::words(name);
        if from.is_empty() || to.is_empty() {
            return Err(vec![
                "source_name y el nombre de la feature no pueden estar vacíos".to_string(),
            ]);
        }
        let to_plural = plural
            .map(names::words)
            .unwrap_or_else(|| names::pluralize(&to));
        let pairs = names::pairs(&from, &names::pluralize(&from), &to, &to_plural);
        let target = PathBuf::from(names::render(&t.target, &to, &to_plural));
        let source = root.join(&t.source);
        if !fs.is_dir(&source) {
            errors.push(format!("no existe la feature dorada {}", t.source));
        }
        if fs.exists(&root.join(&target)) {
            errors.push(format!("ya existe {}", target.display()));
        }

        let mut injects: Vec<(PathBuf, String)> = Vec::new();
        for inj in &t.inject {
            let path = PathBuf::from(&inj.file);
            let current = match injects.iter().position(|(p, _)| *p == path) {
                Some(i) => Some(injects.remove(i).1),
                None => fs
                    .read(&root.join(&path))
                    .ok()
                    .and_then(|b| String::from_utf8(b).ok()),
            };
            let Some(current) = current else {
                errors.push(format!("{}: no existe", inj.file));
                continue;
            };
            match insert_after_marker(
                &current,
                &inj.marker,
                &names::render(&inj.line, &to, &to_plural),
            ) {
                Some(new) => injects.push((path, new)),
                None => errors.push(format!(
                    "{}: falta el marcador \"{}\"",
                    inj.file, inj.marker
                )),
            }
        }

        let mut files = Vec::new();
        if fs.is_dir(&source) {
            for path in fs.walk(&source) {
                let rel = path.strip_prefix(&source).unwrap_or(&path);
                if t.exceptions.iter().any(|e| rel.starts_with(e)) {
                    continue;
                }
                let content = match fs.read(&path) {
                    Ok(c) => c,
                    Err(e) => {
                        errors.push(format!("{}: {e}", path.display()));
                        continue;
                    }
                };
                let content = match String::from_utf8(content) {
                    Ok(s) => names::substitute(&s, &pairs).into_bytes(),
                    Err(e) => e.into_bytes(),
                };
                files.push((
                    target.join(names::substitute(&rel.to_string_lossy(), &pairs)),
                    content,
                ));
            }
            if files.is_empty() {
                errors.push(format!("la feature dorada {} está vacía", t.source));
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        let main = match &t.main {
            Some(m) => Some(target.join(names::substitute(m, &pairs))),
            None => main_file(&files),
        };
        Ok(Plan {
            files,
            injects,
            post: t.post,
            main,
        })
    }
}

/// Inserta `line` en la línea siguiente al marcador, con su misma sangría.
fn insert_after_marker(text: &str, marker: &str, line: &str) -> Option<String> {
    let start = text.split_inclusive('\n').scan(0, |pos, l| {
        let s = *pos;
        *pos += l.len();
        Some((s, l))
    });
    for (s, l) in start {
        if l.contains(marker) {
            let content = l.trim_end_matches(['\n', '\r']);
            let indent = &content[..content.len() - content.trim_start().len()];
            let end = s + content.len();
            return Some(format!("{}\n{indent}{line}{}", &text[..end], &text[end..]));
        }
    }
    None
}

fn main_file(files: &[(PathBuf, Vec<u8>)]) -> Option<PathBuf> {
    const MAIN: [&str; 5] = ["mod", "index", "__init__", "main", "lib"];
    files
        .iter()
        .map(|(p, _)| p)
        .find(|p| p.file_stem().is_some_and(|s| MAIN.iter().any(|m| s == *m)))
        .or(files.first().map(|(p, _)| p))
        .cloned()
}

#[cfg(test)]
mod tests {
    #[test]
    fn injects_after_marker_keeping_indent() {
        let out = super::insert_after_marker("a\n    // m\n    b\n", "// m", ".x()").unwrap();
        assert_eq!(out, "a\n    // m\n    .x()\n    b\n");
        assert!(super::insert_after_marker("a\n", "// m", "x").is_none());
    }
}
