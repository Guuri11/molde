use super::*;
use crate::domain::{body, names};

#[derive(Debug, Clone, PartialEq)]
pub struct Captured {
    pub file: PathBuf,
    /// Línea de la entrada nueva, desde 0.
    pub line: u32,
    pub snippet: Snippet,
    /// Por qué no se ha usado la respuesta de la IA, o qué se ha descartado de ella.
    pub notices: Vec<Notice>,
}

/// Lo que se calcula antes de preguntar a la IA, para no tener el motor bloqueado mientras responde.
#[derive(Debug, Clone)]
pub struct CaptureDraft {
    root: PathBuf,
    file: PathBuf,
    language: String,
    selection: String,
    requires: Vec<String>,
    omitted: Vec<String>,
    pub prompt: String,
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
    /// Con IA configurada, el snippet lo escribe ella; si falla, se generaliza por regla.
    pub fn capture(
        &mut self,
        path: &Path,
        language: &str,
        text: &str,
        first: usize,
        last: usize,
    ) -> Result<Captured, String> {
        let draft = self.prepare_capture(path, language, text, first, last)?;
        let answer = self.assistant().map(|a| a.ask(&draft.prompt));
        self.finish_capture(draft, answer)
    }

    pub fn assistant(&self) -> Option<Arc<dyn Assistant>> {
        self.ports.assistant.clone()
    }

    pub fn prepare_capture(
        &self,
        path: &Path,
        language: &str,
        text: &str,
        first: usize,
        last: usize,
    ) -> Result<CaptureDraft, String> {
        let root = self
            .find_project(path)
            .or_else(|| self.find_marker(path, ".git"))
            .ok_or("no hay proyecto: crea una carpeta .molde en la raíz")?;
        let (selection, start) = select_lines(text, first, last);
        let (requires, omitted) = self.used_imports(language, text, start, &selection);
        let prompt = capture_prompt(
            language,
            &selection,
            &requires,
            &self.ports.syntax.containers(language),
        );
        Ok(CaptureDraft {
            file: root
                .join(".molde/snippets")
                .join(format!("{language}.json")),
            root,
            language: language.to_string(),
            selection,
            requires,
            omitted,
            prompt,
        })
    }

    /// Guarda la entrada. `answer` es la respuesta de la IA, si se le ha preguntado.
    pub fn finish_capture(
        &mut self,
        d: CaptureDraft,
        answer: Option<Result<String, String>>,
    ) -> Result<Captured, String> {
        let existing = self
            .ports
            .snippets
            .load(&d.file)
            .map_err(|e| format!("{}:{}: {}", d.file.display(), e.line, e.message))?
            .unwrap_or_default();
        let free = |k: &str| !existing.iter().any(|s| s.key == k);
        let provisional = (1..)
            .map(|n| format!("captura-{n}"))
            .find(|k| free(k))
            .unwrap_or_default();
        let review = (!d.omitted.is_empty()).then(|| {
            format!(
                "Revisar: imports relativos omitidos: {}",
                d.omitted.join("; ")
            )
        });
        let mut notices = Vec::new();
        let suggested = answer.map(|a| {
            let s = a.and_then(|text| self.ports.snippets.parse_entry(&text))?;
            body::expand(&s.body).map_err(|e| format!("el cuerpo no es válido: {e}"))?;
            Ok::<_, String>(s)
        });
        let snippet = match suggested {
            Some(Ok(mut s)) => {
                if !is_key(&s.key) || !free(&s.key) {
                    s.key = provisional;
                }
                if s.prefixes.iter().all(|p| p.trim().is_empty()) {
                    s.prefixes = vec![s.key.replace('-', " ")];
                }
                let dropped = self.drop_invalid(&d.language, &mut s);
                if !dropped.is_empty() {
                    notices.push(Notice::Warning(format!(
                        "molde: se ha descartado de la respuesta de la IA: {}",
                        dropped.join(", ")
                    )));
                }
                if let Some(r) = review {
                    s.description = [s.description, r]
                        .into_iter()
                        .filter(|t| !t.is_empty())
                        .collect::<Vec<_>>()
                        .join(". ");
                }
                s
            }
            other => {
                if let Some(Err(e)) = other {
                    notices.push(Notice::Warning(format!(
                        "molde: la IA no ha dado un snippet válido ({e}); se ha capturado por regla"
                    )));
                }
                let config = self.config(Some(&d.root), &mut Vec::new());
                Snippet {
                    prefixes: vec![provisional.replace('-', " ")],
                    key: provisional,
                    description: review.unwrap_or_default(),
                    body: self.generalize(&d.language, &d.selection, &config.common_words),
                    requires: d.requires,
                    context: None,
                }
            }
        };
        let line = self.ports.snippets.append(&d.file, &snippet)?;
        Ok(Captured {
            file: d.file,
            line,
            snippet,
            notices,
        })
    }

    /// Quita los `requires` que no son imports del lenguaje y los `context` que no son contenedores.
    fn drop_invalid(&self, language: &str, s: &mut Snippet) -> Vec<String> {
        let syntax = &self.ports.syntax;
        let mut dropped = Vec::new();
        if !syntax.supports(language) {
            return dropped;
        }
        s.requires.retain(|r| {
            let ok = syntax.parse_requirements(language, r).is_ok();
            if !ok {
                dropped.push(format!("requires \"{r}\""));
            }
            ok
        });
        let containers = syntax.containers(language);
        if let Some(context) = &mut s.context {
            context.retain(|c| {
                let ok = containers.contains(c);
                if !ok {
                    dropped.push(format!("context \"{c}\""));
                }
                ok
            });
            if context.is_empty() {
                s.context = None;
            }
        }
        dropped
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

/// Claves en kebab-case ASCII, como `use-case-delete`.
fn is_key(k: &str) -> bool {
    !k.is_empty()
        && k.split('-').all(|p| {
            !p.is_empty()
                && p.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

/// Lo que se le pide a la IA: la selección, sus imports y los contenedores del lenguaje. Nada más del fichero.
pub fn capture_prompt(
    language: &str,
    selection: &str,
    requires: &[String],
    containers: &[String],
) -> String {
    let list = |items: &[String]| {
        if items.is_empty() {
            "ninguno".to_string()
        } else {
            items
                .iter()
                .map(|i| format!("`{i}`"))
                .collect::<Vec<_>>()
                .join(", ")
        }
    };
    format!(
        "Convierte este fragmento de {language} en un snippet para molde, un servidor de snippets.\n\
         No uses herramientas ni leas ficheros: todo lo que necesitas está en este mensaje.\n\
         \n\
         Devuelve solo un objeto JSON, sin explicaciones, con estos campos:\n\
         - \"key\": identificador en kebab-case ASCII, como \"use-case-delete\".\n\
         - \"prefix\": las palabras que alguien escribiría para buscarlo, como \"use case delete\".\n\
         - \"description\": una frase que diga qué genera.\n\
         - \"body\": el cuerpo, como lista de líneas, con la sintaxis de snippets de VSCode.\n\
         - \"requires\": las sentencias de import completas que necesita el cuerpo.\n\
         - \"context\": dónde tiene sentido ofrecerlo, de entre {containers}. Omítelo si sirve en cualquier sitio.\n\
         \n\
         Cómo escribir el cuerpo:\n\
         - Déjalo lo más limpio posible: quita los logs, los comentarios y los textos propios de este caso, y conserva la estructura.\n\
         - Los nombres de dominio pasan a ser huecos, como ${{1:Invoice}}. Donde se repite el mismo nombre, el mismo número: ${{1}}. \
         Cada variante de mayúsculas lleva su propio hueco: ${{1:Invoice}} y ${{2:invoice}}.\n\
         - Termina con $0 donde deba quedar el cursor.\n\
         - En el texto fijo, escapa con una barra `$`, `}}` y `\\`: `\\$`, `\\}}`, `\\\\`. Dentro del JSON, esa barra se escribe doble.\n\
         - Los imports van en \"requires\", no en el cuerpo. El fragmento usa {requires}: \
         quita los que solo sirven para este caso y añade los que falten.\n\
         - \"requires\" es texto fijo, sin huecos: un import que depende de un nombre de dominio se deja fuera.\n\
         \n\
         Fragmento:\n\
         ```{language}\n\
         {selection}\n\
         ```\n",
        containers = list(containers),
        requires = list(requires),
    )
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
    fn keys_are_ascii_kebab_case() {
        assert!(is_key("use-case-delete") && is_key("v2"));
        for k in ["", "Get By Id!", "a--b", "-a", "camión"] {
            assert!(!is_key(k), "{k}");
        }
    }

    #[test]
    fn selects_full_lines_without_common_indent() {
        let (sel, start) = select_lines("a\n    b\n      c\n    d\n", 1, 3);
        assert_eq!(sel, "b\n  c\nd");
        assert_eq!(start, 2);
    }
}
