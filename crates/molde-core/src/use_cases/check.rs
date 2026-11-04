use super::*;
use crate::domain::body;

impl Engine {
    /// Errores de los ficheros de snippets, la configuración y las plantillas. Vacío si todo está bien.
    pub fn check(&mut self, root: Option<&Path>) -> Vec<String> {
        let mut errors = Vec::new();
        for (_, _, dir) in self.snippet_dirs(root) {
            for lang in self.snippet_languages(root) {
                let path = dir.join(format!("{lang}.json"));
                let snippets = match self.ports.snippets.load(&path) {
                    Ok(Some(s)) => s,
                    Ok(None) => continue,
                    Err(e) => {
                        errors.push(format!("{}:{}: {}", path.display(), e.line, e.message));
                        continue;
                    }
                };
                for s in snippets {
                    let at = format!("{}: {}", path.display(), s.key);
                    if let Err(e) = body::expand(&s.body) {
                        errors.push(format!("{at}: body: {e}"));
                    }
                    if self.ports.syntax.supports(&lang) {
                        for r in &s.requires {
                            if let Err(e) = self.ports.syntax.parse_requirements(&lang, r) {
                                errors.push(format!("{at}: requires \"{r}\": {e}"));
                            }
                        }
                    }
                }
            }
        }
        let Some(root) = root else { return errors };
        let config = root.join(".molde/molde.toml");
        if let Ok(text) = self.ports.fs.read(&config)
            && let Err(e) = Config::parse(&String::from_utf8_lossy(&text))
        {
            errors.push(format!("{}: {e}", config.display()));
        }
        let templates = root.join(".molde/templates");
        for t in self.ports.fs.walk(&templates) {
            if t.file_name().is_some_and(|n| n == "template.toml") {
                let name = t
                    .parent()
                    .and_then(Path::file_name)
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                errors.extend(
                    self.check_template(root, &name)
                        .into_iter()
                        .map(|e| format!("plantilla {name}: {e}")),
                );
            }
        }
        errors
    }
}
