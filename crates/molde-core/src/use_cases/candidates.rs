use super::*;
use crate::domain::body;

#[derive(Debug, Clone, PartialEq)]
pub struct Location {
    /// Relativa a la raíz del proyecto.
    pub path: PathBuf,
    /// Filas desde 0, ambas incluidas.
    pub start_line: usize,
    pub end_line: usize,
    start_byte: usize,
    end_byte: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub language: String,
    pub locations: Vec<Location>,
}

/// Lo trivial no merece snippet.
pub const MIN_LINES: usize = 5;
/// La regla de tres.
pub const MIN_REPEATS: usize = 3;

impl Engine {
    /// Patrones de al menos 5 líneas repetidos 3 o más veces que ningún snippet cubre.
    pub fn candidates(&mut self, root: &Path) -> (Vec<Candidate>, Vec<Notice>) {
        let mut notices = Vec::new();
        let config = self.config(Some(root), &mut notices);
        let mut order: Vec<(String, u64)> = Vec::new();
        let mut groups: HashMap<(String, u64), Vec<Location>> = HashMap::new();
        for path in self.ports.fs.walk(root) {
            let Some(lang) = self.ports.syntax.language_for_path(&path) else {
                continue;
            };
            let Ok(bytes) = self.ports.fs.read(&path) else {
                continue;
            };
            if bytes.len() as u64 > config.max_file_size_mb * MB {
                continue;
            }
            let Ok(text) = String::from_utf8(bytes) else {
                continue;
            };
            let rel = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
            for f in self.ports.syntax.fragments(&lang, &text, MIN_LINES) {
                let key = (lang.clone(), f.hash);
                let locs = groups.entry(key.clone()).or_default();
                if locs.is_empty() {
                    order.push(key);
                }
                locs.push(Location {
                    path: rel.clone(),
                    start_line: f.start_line,
                    end_line: f.end_line,
                    start_byte: f.start_byte,
                    end_byte: f.end_byte,
                });
            }
        }
        let repeated: Vec<((String, u64), Vec<Location>)> = order
            .into_iter()
            .filter_map(|k| groups.remove(&k).map(|v| (k, v)))
            .filter(|(_, v)| v.len() >= MIN_REPEATS)
            .collect();

        let mut covered: HashSet<(String, u64)> = HashSet::new();
        let languages: HashSet<String> = repeated.iter().map(|((l, _), _)| l.clone()).collect();
        for lang in languages {
            for (_, s) in self.effective(Some(root), std::slice::from_ref(&lang), &mut notices) {
                let Ok(code) = body::expand(&s.body) else {
                    continue;
                };
                covered.extend(
                    self.ports
                        .syntax
                        .fragments(&lang, &code, 1)
                        .into_iter()
                        .map(|f| (lang.clone(), f.hash)),
                );
            }
        }

        let mut out: Vec<Candidate> = repeated
            .iter()
            .filter(|(k, locs)| !covered.contains(k) && !subsumed(k, locs, &repeated))
            .map(|((lang, _), locs)| Candidate {
                language: lang.clone(),
                locations: locs.clone(),
            })
            .collect();
        out.sort_by_key(|c| {
            (
                std::cmp::Reverse(c.locations.len()),
                std::cmp::Reverse(c.lines()),
            )
        });
        (out, notices)
    }

    /// Guarda el candidato como snippet con la misma generalización que la captura.
    pub fn save_candidate(
        &mut self,
        root: &Path,
        candidate: &Candidate,
    ) -> Result<Captured, String> {
        let loc = &candidate.locations[0];
        let path = root.join(&loc.path);
        let text = self
            .ports
            .fs
            .read(&path)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        self.capture(
            &path,
            &candidate.language,
            &String::from_utf8_lossy(&text),
            loc.start_line,
            loc.end_line,
        )
    }
}

impl Candidate {
    pub fn lines(&self) -> usize {
        self.locations
            .first()
            .map_or(0, |l| l.end_line - l.start_line + 1)
    }
}

/// Un grupo que solo aparece dentro de las apariciones de otro mayor no se propone aparte.
fn subsumed(
    key: &(String, u64),
    locs: &[Location],
    all: &[((String, u64), Vec<Location>)],
) -> bool {
    all.iter().any(|(k, outer)| {
        k != key
            && locs.iter().all(|l| {
                outer.iter().any(|o| {
                    o.path == l.path
                        && o.start_byte <= l.start_byte
                        && l.end_byte <= o.end_byte
                        && o.end_byte - o.start_byte > l.end_byte - l.start_byte
                })
            })
    })
}

pub fn candidates_markdown(candidates: &[Candidate]) -> String {
    let mut out = String::from("# molde candidates\n\n");
    if candidates.is_empty() {
        out.push_str(&format!(
            "Sin candidatos: no hay patrones de {MIN_LINES} o más líneas repetidos {MIN_REPEATS} o más veces sin un snippet que los cubra.\n"
        ));
    }
    for (i, c) in candidates.iter().enumerate() {
        out.push_str(&format!(
            "## {}. {}, {} apariciones de {} líneas\n\n",
            i + 1,
            c.language,
            c.locations.len(),
            c.lines()
        ));
        for l in &c.locations {
            out.push_str(&format!(
                "- {}:{}-{}\n",
                l.path.display(),
                l.start_line + 1,
                l.end_line + 1
            ));
        }
        out.push('\n');
    }
    out
}
