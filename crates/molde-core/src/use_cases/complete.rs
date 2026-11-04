use super::*;

pub struct CompletionRequest<'a> {
    pub path: &'a Path,
    /// languageId del editor: "rust", "typescriptreact"...
    pub language: &'a str,
    pub text: &'a str,
    /// Byte del cursor.
    pub offset: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompletionItem {
    pub id: SnippetId,
    pub project_root: String,
    pub label: String,
    pub description: String,
    pub body: String,
    /// Byte donde empieza lo que se ha escrito del prefijo; se sustituye hasta el cursor.
    pub replace_start: usize,
    /// Byte y texto de los imports que faltan.
    pub import_edit: Option<(usize, String)>,
    pub sort_text: String,
}

/// Prioridad añadida a los snippets con context cuando el árbol no deja decidir.
const UNDECIDED: u32 = 1000;

impl Engine {
    pub fn complete(&mut self, req: &CompletionRequest) -> (Vec<CompletionItem>, Vec<Notice>) {
        let mut notices = Vec::new();
        let root = self.find_project(req.path);
        let config = self.config(root.as_deref(), &mut notices);
        let mut languages = vec![req.language.to_string()];
        languages.extend(self.ports.syntax.extra_snippet_languages(req.language));
        languages.push("all".to_string());
        let snippets = self.effective(root.as_deref(), &languages, &mut notices);

        let supported = self.ports.syntax.supports(req.language);
        if !supported && self.warned.insert(req.language.to_string()) {
            notices.push(Notice::Log(format!(
                "molde: {} no tiene gramática de Treesitter, así que no hay soporte de contexto ni de imports",
                req.language
            )));
        }
        let analyze = supported && req.text.len() as u64 <= config.max_file_size_mb * MB;
        let syntax = &self.ports.syntax;
        let context = analyze.then(|| syntax.container(req.language, req.text, req.offset));
        let mut scope: Option<ImportScope> = None;
        let mut parsed: HashMap<String, Vec<Requirement>> = HashMap::new();
        let replace_start = word_start(req.text, req.offset);
        let project_root = root.map(|r| r.display().to_string()).unwrap_or_default();

        let mut items = Vec::new();
        for (id, s) in snippets {
            let mut priority = config.sort_priority;
            if let (Some(ctx), Some(allowed)) = (&context, &s.context) {
                match ctx {
                    Context::Known(kind) if !allowed.contains(kind) => continue,
                    Context::Known(_) => {}
                    Context::Unknown => priority += UNDECIDED,
                }
            }
            let import_edit = if analyze && !s.requires.is_empty() {
                let scope =
                    scope.get_or_insert_with(|| syntax.imports(req.language, req.text, req.offset));
                missing_imports(
                    syntax.as_ref(),
                    req.language,
                    &s.requires,
                    scope,
                    &mut parsed,
                )
            } else {
                None
            };
            for label in &s.prefixes {
                items.push(CompletionItem {
                    id: id.clone(),
                    project_root: project_root.clone(),
                    label: label.clone(),
                    description: s.description.clone(),
                    body: s.body.clone(),
                    replace_start,
                    import_edit: import_edit.clone(),
                    sort_text: format!("{priority:04}{label}"),
                });
            }
        }
        (items, notices)
    }
}

/// Imports de `requires` que faltan, nombre a nombre. Si falta la sentencia entera se copia tal cual.
fn missing_imports(
    syntax: &dyn SyntaxAnalyzer,
    language: &str,
    requires: &[String],
    scope: &ImportScope,
    parsed: &mut HashMap<String, Vec<Requirement>>,
) -> Option<(usize, String)> {
    let mut have: HashSet<String> = scope.imports.iter().map(|r| r.name.clone()).collect();
    let mut lines: Vec<String> = Vec::new();
    for req in requires {
        let req = req.trim();
        // Muchos snippets repiten los mismos imports: cada sentencia se descompone una vez por petición.
        let names = parsed
            .entry(req.to_string())
            .or_insert_with(|| syntax.parse_requirements(language, req).unwrap_or_default());
        match names {
            names if !names.is_empty() => {
                let missing: Vec<&Requirement> =
                    names.iter().filter(|r| !have.contains(&r.name)).collect();
                if missing.len() == names.len() {
                    lines.push(req.to_string());
                } else {
                    lines.extend(missing.iter().map(|r| r.single.clone()));
                }
                have.extend(missing.iter().map(|r| r.name.clone()));
            }
            // Sin descomposición: se compara la sentencia entera.
            _ => {
                if !scope.statements.iter().any(|s| s.trim() == req)
                    && !lines.iter().any(|l| l == req)
                {
                    lines.push(req.to_string());
                }
            }
        }
    }
    if lines.is_empty() {
        return None;
    }
    let mut text = String::new();
    if scope.newline_before {
        text.push('\n');
    }
    for l in lines {
        text.push_str(&scope.indent);
        text.push_str(&l);
        text.push('\n');
    }
    if !scope.has_imports {
        text.push('\n');
    }
    Some((scope.insert_at, text))
}

/// Inicio de la palabra que se está escribiendo. Incluye `-` porque hay prefijos como "ddd-model".
pub(crate) fn word_start(text: &str, offset: usize) -> usize {
    let mut offset = offset.min(text.len());
    while !text.is_char_boundary(offset) {
        offset -= 1;
    }
    text[..offset]
        .char_indices()
        .rev()
        .take_while(|(_, c)| c.is_alphanumeric() || *c == '_' || *c == '-')
        .last()
        .map_or(offset, |(i, _)| i)
}
