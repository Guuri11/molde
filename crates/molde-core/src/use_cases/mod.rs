//! Casos de uso: complete, record_use, capture, stats, candidates, scaffold y check.

mod candidates;
mod capture;
mod check;
mod complete;
mod record_use;
mod scaffold;
mod stats;

pub use candidates::*;
pub use capture::*;
pub use complete::*;
pub use scaffold::*;
pub use stats::*;

use crate::domain::*;
use crate::ports::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub(crate) const DAY: i64 = 86_400;
pub(crate) const MB: u64 = 1024 * 1024;

pub struct Ports {
    pub snippets: Box<dyn SnippetRepository>,
    pub usage: Box<dyn UsageStore>,
    pub syntax: Box<dyn SyntaxAnalyzer>,
    pub clock: Box<dyn Clock>,
    pub fs: Box<dyn FileSystem>,
    pub shell: Box<dyn Shell>,
    /// None si no hay IA configurada: la captura generaliza por regla.
    pub assistant: Option<Arc<dyn Assistant>>,
}

pub struct Engine {
    pub ports: Ports,
    /// `~/.config/molde/snippets`
    pub user_dir: PathBuf,
    catalog: HashMap<PathBuf, Cached>,
    warned: HashSet<String>,
}

struct Cached {
    stamp: Option<Stamp>,
    snippets: Arc<Vec<Snippet>>,
}

impl Engine {
    pub fn new(ports: Ports, user_dir: PathBuf) -> Self {
        Engine {
            ports,
            user_dir,
            catalog: HashMap::new(),
            warned: HashSet::new(),
        }
    }

    /// La carpeta que contiene el `.molde` más cercano subiendo desde `path`.
    pub fn find_project(&self, path: &Path) -> Option<PathBuf> {
        self.find_marker(path, ".molde")
    }

    pub(crate) fn find_marker(&self, path: &Path, marker: &str) -> Option<PathBuf> {
        let start = if self.ports.fs.is_dir(path) {
            path
        } else {
            path.parent()?
        };
        start
            .ancestors()
            .find(|d| self.ports.fs.exists(&d.join(marker)))
            .map(Path::to_path_buf)
    }

    pub(crate) fn config(&self, root: Option<&Path>, notices: &mut Vec<Notice>) -> Config {
        let Some(root) = root else {
            return Config::default();
        };
        let path = root.join(".molde/molde.toml");
        let Ok(bytes) = self.ports.fs.read(&path) else {
            return Config::default();
        };
        Config::parse(&String::from_utf8_lossy(&bytes)).unwrap_or_else(|e| {
            notices.push(Notice::Warning(format!("{}: {e}", path.display())));
            Config::default()
        })
    }

    /// Carpetas de snippets de cada capa: (capa, scope, carpeta).
    pub(crate) fn snippet_dirs(&self, root: Option<&Path>) -> Vec<(Layer, String, PathBuf)> {
        let mut dirs = vec![(Layer::User, String::new(), self.user_dir.clone())];
        if let Some(r) = root {
            dirs.push((
                Layer::Project,
                r.display().to_string(),
                r.join(".molde/snippets"),
            ));
        }
        dirs
    }

    /// Lenguajes con fichero de snippets en alguna capa.
    pub(crate) fn snippet_languages(&self, root: Option<&Path>) -> Vec<String> {
        let mut langs: Vec<String> = Vec::new();
        for (_, _, dir) in self.snippet_dirs(root) {
            for f in self.ports.fs.walk(&dir) {
                if f.parent() == Some(dir.as_path()) && f.extension().is_some_and(|e| e == "json") {
                    let stem = f
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    if !langs.contains(&stem) {
                        langs.push(stem);
                    }
                }
            }
        }
        langs.sort();
        langs
    }

    /// Snippets de un fichero. Solo se vuelve a leer si ha cambiado; si no es válido,
    /// se sigue usando la última versión válida y se avisa con el fichero y la línea.
    pub(crate) fn load(
        &mut self,
        layer: Layer,
        scope: &str,
        language: &str,
        path: &Path,
        notices: &mut Vec<Notice>,
    ) -> Arc<Vec<Snippet>> {
        let stamp = self.ports.fs.stamp(path);
        if let Some(c) = self.catalog.get(path)
            && c.stamp == stamp
        {
            return c.snippets.clone();
        }
        let snippets = match self.ports.snippets.load(path) {
            Ok(s) => {
                let s = Arc::new(s.unwrap_or_default());
                let ids: Vec<SnippetId> = s
                    .iter()
                    .map(|sn| SnippetId {
                        layer,
                        scope: scope.to_string(),
                        language: language.to_string(),
                        key: sn.key.clone(),
                    })
                    .collect();
                if !ids.is_empty()
                    && let Err(e) = self.ports.usage.register(&ids, self.ports.clock.now())
                {
                    notices.push(Notice::Log(format!(
                        "molde: no se pudo guardar la fecha de alta: {e}"
                    )));
                }
                s
            }
            Err(e) => {
                notices.push(Notice::Warning(format!(
                    "{}:{}: {}",
                    path.display(),
                    e.line,
                    e.message
                )));
                self.catalog
                    .get(path)
                    .map(|c| c.snippets.clone())
                    .unwrap_or_default()
            }
        };
        self.catalog.insert(
            path.to_path_buf(),
            Cached {
                stamp,
                snippets: snippets.clone(),
            },
        );
        snippets
    }

    /// Snippets que se aplican: la capa de proyecto sustituye a la de usuario con la misma clave y lenguaje.
    pub(crate) fn effective(
        &mut self,
        root: Option<&Path>,
        languages: &[String],
        notices: &mut Vec<Notice>,
    ) -> Vec<(SnippetId, Snippet)> {
        let mut out = Vec::new();
        let mut seen: HashSet<(String, String)> = HashSet::new();
        for (layer, scope, dir) in self.snippet_dirs(root).into_iter().rev() {
            for lang in languages {
                let path = dir.join(format!("{lang}.json"));
                for s in self.load(layer, &scope, lang, &path, notices).iter() {
                    if seen.insert((lang.clone(), s.key.clone())) {
                        let id = SnippetId {
                            layer,
                            scope: scope.clone(),
                            language: lang.clone(),
                            key: s.key.clone(),
                        };
                        out.push((id, s.clone()));
                    }
                }
            }
        }
        out
    }
}
