use super::*;
use crate::domain::{body, names};

#[derive(Debug, Clone, PartialEq)]
pub struct Captured {
    pub file: PathBuf,
    /// Línea de la entrada nueva, desde 0.
    pub line: u32,
    pub snippet: Snippet,
}

/// Palabras que nunca son términos de dominio, además de `common_words`.
const COMMON: &[&str] = &[
    "self", "this", "cls", "new", "get", "set", "id", "ids", "from", "into", "to", "as", "is",
    "has", "of", "in", "on", "at", "by", "for", "with", "and", "or", "not", "the", "ok", "err",
    "error", "result", "value", "values", "data", "item", "items", "list", "map", "key", "keys",
    "name", "len", "str", "string", "int", "bool", "test", "tests", "args", "kwargs", "default",
    "init", "main", "fn", "def",
];

impl Engine {
    /// Guarda las líneas `first..=last` de `text` como snippet del proyecto.
    pub fn capture(
        &mut self,
        path: &Path,
        language: &str,
        text: &str,
        first: usize,
        last: usize,
    ) -> Result<Captured, String> {
        let root = self
            .find_project(path)
            .or_else(|| self.find_marker(path, ".git"))
            .ok_or("no hay proyecto: crea una carpeta .molde en la raíz")?;
        let config = self.config(Some(&root), &mut Vec::new());
        let (selection, start) = select_lines(text, first, last);
        let body = self.generalize(language, &selection, &config.common_words);
        let (requires, omitted) = self.used_imports(language, text, start, &selection);

        let file = root
            .join(".molde/snippets")
            .join(format!("{language}.json"));
        let existing = self
            .ports
            .snippets
            .load(&file)
            .map_err(|e| format!("{}:{}: {}", file.display(), e.line, e.message))?
            .unwrap_or_default();
        let key = (1..)
            .map(|n| format!("captura-{n}"))
            .find(|k| !existing.iter().any(|s| &s.key == k))
            .unwrap_or_default();
        let description = if omitted.is_empty() {
            String::new()
        } else {
            format!(
                "Revisar: imports relativos omitidos: {}",
                omitted.join("; ")
            )
        };
        let snippet = Snippet {
            prefixes: vec![key.replace('-', " ")],
            key,
            description,
            body,
            requires,
            context: None,
        };
        let line = self.ports.snippets.append(&file, &snippet)?;
        Ok(Captured {
            file,
            line,
            snippet,
        })
    }

    /// Cuerpo de snippet para `selection`: términos de dominio y literales pasan a ser placeholders.
    pub fn generalize(&self, language: &str, selection: &str, common_words: &[String]) -> String {
        let tokens = if self.ports.syntax.supports(language) {
            self.ports.syntax.tokens(language, selection)
        } else {
            Vec::new()
        };
        generalize(selection, &tokens, common_words)
    }

    /// Imports del fichero que usa la selección. Los relativos se omiten y se devuelven aparte.
    fn used_imports(
        &self,
        language: &str,
        text: &str,
        offset: usize,
        selection: &str,
    ) -> (Vec<String>, Vec<String>) {
        let syntax = &self.ports.syntax;
        if !syntax.supports(language) {
            return (Vec::new(), Vec::new());
        }
        let used: HashSet<&str> = syntax
            .tokens(language, selection)
            .into_iter()
            .filter(|t| t.kind == TokenKind::Ident)
            .map(|t| &selection[t.start..t.end])
            .collect();
        let scope = syntax.imports(language, text, offset);
        let (mut requires, mut omitted) = (Vec::new(), Vec::new());
        let mut starts: Vec<usize> = scope.imports.iter().map(|r| r.start).collect();
        starts.dedup();
        for start in starts {
            let group: Vec<&Requirement> =
                scope.imports.iter().filter(|r| r.start == start).collect();
            // En Python `import os.path` deja disponible `os`.
            let hits: Vec<&&Requirement> = group
                .iter()
                .filter(|r| used.contains(r.name.split('.').next().unwrap_or("")))
                .collect();
            if hits.is_empty() {
                continue;
            }
            if group[0].relative {
                omitted.push(group[0].statement.clone());
            } else if hits.len() == group.len() {
                requires.push(group[0].statement.clone());
            } else {
                requires.extend(hits.iter().map(|r| r.single.clone()));
            }
        }
        (requires, omitted)
    }
}

/// Líneas completas `first..=last` sin la sangría común, y el byte donde empiezan.
fn select_lines(text: &str, first: usize, last: usize) -> (String, usize) {
    let start: usize = text.split_inclusive('\n').take(first).map(str::len).sum();
    let lines: Vec<&str> = text
        .lines()
        .skip(first)
        .take(last.saturating_sub(first) + 1)
        .collect();
    let indent = lines
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| &l[..l.len() - l.trim_start().len()])
        .reduce(|a, b| {
            let n = a.bytes().zip(b.bytes()).take_while(|(x, y)| x == y).count();
            &a[..n]
        })
        .unwrap_or("");
    let body: Vec<&str> = lines
        .iter()
        .map(|l| l.strip_prefix(indent).unwrap_or(l.trim_start()))
        .collect();
    (body.join("\n"), start)
}

/// Un término es de dominio si aparece dentro de al menos dos identificadores distintos y no es
/// vocabulario común. Cada variante de mayúsculas es un placeholder propio, porque no todos los
/// clientes aplican transformaciones. Los literales pasan a ser tabstops con su valor.
pub fn generalize(text: &str, tokens: &[Token], common_words: &[String]) -> String {
    let common: HashSet<String> = COMMON
        .iter()
        .map(|s| s.to_string())
        .chain(common_words.iter().map(|w| w.to_lowercase()))
        .collect();
    let mut seen: HashMap<String, HashSet<&str>> = HashMap::new();
    for t in tokens.iter().filter(|t| t.kind == TokenKind::Ident) {
        let ident = &text[t.start..t.end];
        for r in names::parts(ident) {
            seen.entry(ident[r].to_lowercase())
                .or_default()
                .insert(ident);
        }
    }
    let domain: HashSet<String> = seen
        .into_iter()
        .filter(|(term, ids)| {
            ids.len() >= 2
                && term.len() >= 2
                && !common.contains(term)
                && !term.bytes().all(|b| b.is_ascii_digit())
        })
        .map(|(term, _)| term)
        .collect();

    // (inicio, fin, es literal)
    let mut spans: Vec<(usize, usize, bool)> = Vec::new();
    for t in tokens {
        match t.kind {
            TokenKind::Literal if t.end > t.start => spans.push((t.start, t.end, true)),
            TokenKind::Literal => {}
            TokenKind::Ident => {
                let ident = &text[t.start..t.end];
                for r in names::parts(ident) {
                    if domain.contains(&ident[r.clone()].to_lowercase()) {
                        spans.push((t.start + r.start, t.start + r.end, false));
                    }
                }
            }
        }
    }
    spans.sort();

    let mut out = String::new();
    let mut pos = 0;
    let mut next = 1;
    let mut variants: HashMap<&str, u32> = HashMap::new();
    for (s, e, literal) in spans {
        if s < pos {
            continue;
        }
        out.push_str(&body::escape(&text[pos..s]));
        let value = &text[s..e];
        match variants.get(value) {
            Some(n) if !literal => out.push_str(&format!("${{{n}}}")),
            _ => {
                if !literal {
                    variants.insert(value, next);
                }
                out.push_str(&format!("${{{next}:{}}}", body::escape(value)));
                next += 1;
            }
        }
        pos = e;
    }
    out.push_str(&body::escape(&text[pos..]));
    out.push_str("$0");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_full_lines_without_common_indent() {
        let (sel, start) = select_lines("a\n    b\n      c\n    d\n", 1, 3);
        assert_eq!(sel, "b\n  c\nd");
        assert_eq!(start, 2);
    }
}
