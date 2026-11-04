//! Adaptador Treesitter: contexto, imports, generalización y huellas estructurales.
//!
//! Un lenguaje es su gramática más dos consultas en `queries/<lenguaje>/`:
//! `containers.scm` (@container) e `imports.scm` (@import, @header, @scope).
//! Descomponer cada import nombre a nombre es un extra por lenguaje; sin él,
//! los imports se comparan por la sentencia entera.

use molde_core::*;
use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::Range;
use std::path::Path;
use std::sync::Mutex;
use tree_sitter::{
    InputEdit, Language, Node, Parser, Point, Query, QueryCursor, StreamingIterator, Tree,
};

#[derive(Clone, Copy, PartialEq)]
enum Decompose {
    Rust,
    TypeScript,
    Python,
    None,
}

struct Lang {
    id: String,
    extensions: Vec<String>,
    language: Language,
    containers: Query,
    imports: Query,
    decompose: Decompose,
}

pub struct Treesitter {
    langs: Vec<Lang>,
    /// Últimos documentos parseados. Contexto e imports comparten árbol, y el siguiente
    /// texto del mismo lenguaje se parsea de forma incremental a partir del anterior.
    cache: Mutex<Vec<Parsed>>,
}

struct Parsed {
    lang: usize,
    hash: u64,
    text: String,
    tree: Tree,
}

macro_rules! queries {
    ($dir:literal) => {
        (
            include_str!(concat!("../../../queries/", $dir, "/containers.scm")),
            include_str!(concat!("../../../queries/", $dir, "/imports.scm")),
        )
    };
}

const LITERALS: &[&str] = &[
    "string_content",
    "string_fragment",
    "integer_literal",
    "float_literal",
    "integer",
    "float",
    "number",
];

impl Default for Treesitter {
    fn default() -> Self {
        Self::new()
    }
}

impl Treesitter {
    /// Rust, TypeScript, TSX y Python.
    pub fn new() -> Self {
        let mut t = Treesitter {
            langs: Vec::new(),
            cache: Mutex::default(),
        };
        let builtin: [(&str, &[&str], Language, (&str, &str), Decompose); 4] = [
            (
                "rust",
                &["rs"],
                tree_sitter_rust::LANGUAGE.into(),
                queries!("rust"),
                Decompose::Rust,
            ),
            (
                "typescript",
                &["ts", "mts", "cts"],
                tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
                queries!("typescript"),
                Decompose::TypeScript,
            ),
            (
                "typescriptreact",
                &["tsx"],
                tree_sitter_typescript::LANGUAGE_TSX.into(),
                queries!("tsx"),
                Decompose::TypeScript,
            ),
            (
                "python",
                &["py", "pyi"],
                tree_sitter_python::LANGUAGE.into(),
                queries!("python"),
                Decompose::Python,
            ),
        ];
        for (id, ext, language, (containers, imports), decompose) in builtin {
            t.add(id, ext, language, containers, imports, decompose)
                .expect("las consultas incluidas son válidas");
        }
        t
    }

    /// Añade un lenguaje solo con su gramática y sus consultas.
    pub fn with_language(
        mut self,
        id: &str,
        extensions: &[&str],
        language: Language,
        containers: &str,
        imports: &str,
    ) -> Result<Self, String> {
        self.add(
            id,
            extensions,
            language,
            containers,
            imports,
            Decompose::None,
        )?;
        Ok(self)
    }

    fn add(
        &mut self,
        id: &str,
        extensions: &[&str],
        language: Language,
        containers: &str,
        imports: &str,
        decompose: Decompose,
    ) -> Result<(), String> {
        let containers =
            Query::new(&language, containers).map_err(|e| format!("{id}/containers.scm: {e}"))?;
        let imports =
            Query::new(&language, imports).map_err(|e| format!("{id}/imports.scm: {e}"))?;
        let extensions = extensions.iter().map(|e| e.to_string()).collect();
        self.langs.push(Lang {
            id: id.to_string(),
            extensions,
            language,
            containers,
            imports,
            decompose,
        });
        Ok(())
    }

    fn lang(&self, id: &str) -> Option<usize> {
        self.langs.iter().position(|l| l.id == id)
    }

    fn parse(&self, i: usize, text: &str, old: Option<&Tree>) -> Tree {
        let mut parser = Parser::new();
        parser
            .set_language(&self.langs[i].language)
            .expect("gramática compatible con tree-sitter");
        parser.parse(text, old).expect("el parser tiene lenguaje")
    }

    fn tree(&self, i: usize, text: &str) -> Tree {
        let mut h = DefaultHasher::new();
        text.hash(&mut h);
        let hash = h.finish();
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(p) = cache.iter().find(|p| p.lang == i && p.hash == hash) {
            return p.tree.clone();
        }
        // El cliente manda el texto entero en cada cambio: el cambio es lo que no coincide por los extremos.
        let tree = match cache.iter().find(|p| p.lang == i) {
            Some(prev) => {
                let mut old = prev.tree.clone();
                old.edit(&diff(&prev.text, text));
                self.parse(i, text, Some(&old))
            }
            None => self.parse(i, text, None),
        };
        cache.insert(
            0,
            Parsed {
                lang: i,
                hash,
                text: text.to_string(),
                tree: tree.clone(),
            },
        );
        cache.truncate(4);
        tree
    }
}

/// La edición que convierte `old` en `new`: lo que queda entre el prefijo y el sufijo comunes.
fn diff(old: &str, new: &str) -> InputEdit {
    let mut prefix = old
        .bytes()
        .zip(new.bytes())
        .take_while(|(a, b)| a == b)
        .count();
    while !new.is_char_boundary(prefix) {
        prefix -= 1;
    }
    let room = old.len().min(new.len()) - prefix;
    let mut suffix = old
        .bytes()
        .rev()
        .zip(new.bytes().rev())
        .take(room)
        .take_while(|(a, b)| a == b)
        .count();
    while !new.is_char_boundary(new.len() - suffix) || !old.is_char_boundary(old.len() - suffix) {
        suffix -= 1;
    }
    let point = |s: &str, at: usize| {
        let before = &s.as_bytes()[..at];
        let line_start = before
            .iter()
            .rposition(|&b| b == b'\n')
            .map_or(0, |i| i + 1);
        Point {
            row: before.iter().filter(|&&b| b == b'\n').count(),
            column: at - line_start,
        }
    };
    let (old_end, new_end) = (old.len() - suffix, new.len() - suffix);
    InputEdit {
        start_byte: prefix,
        old_end_byte: old_end,
        new_end_byte: new_end,
        start_position: point(old, prefix),
        old_end_position: point(old, old_end),
        new_end_position: point(new, new_end),
    }
}

/// Nodos capturados por `query`, con el nombre de su captura.
fn captured<'t>(
    query: &Query,
    root: Node<'t>,
    text: &str,
    range: Option<Range<usize>>,
) -> Vec<(String, Node<'t>)> {
    let mut qc = QueryCursor::new();
    if let Some(r) = range {
        qc.set_byte_range(r);
    }
    let mut out = Vec::new();
    let mut matches = qc.matches(query, root, text.as_bytes());
    while let Some(m) = matches.next() {
        for c in m.captures() {
            out.push((query.capture_names()[c.index as usize].to_string(), c.node));
        }
    }
    out
}

fn ids(caps: &[(String, Node)], name: &str) -> HashSet<usize> {
    caps.iter()
        .filter(|(n, _)| n == name)
        .map(|(_, node)| node.id())
        .collect()
}

/// Donde mirar en el árbol: el inicio de la palabra que se está escribiendo, o el cursor.
fn probe(text: &str, offset: usize) -> usize {
    let mut offset = offset.min(text.len());
    while !text.is_char_boundary(offset) {
        offset -= 1;
    }
    text[..offset]
        .char_indices()
        .rev()
        .take_while(|(_, c)| c.is_alphanumeric() || *c == '_')
        .last()
        .map_or(offset, |(i, _)| i)
}

/// Nodo del cursor. Treesitter cuelga los comentarios donde le conviene (en Python, el de la
/// última línea de una función queda en el módulo), así que si hay código antes en la línea, decide ese código.
fn node_at<'t>(root: Node<'t>, text: &str, at: usize) -> Node<'t> {
    let mut node = root.descendant_for_byte_range(at, at).unwrap_or(root);
    // Justo al final de un comentario, el cursor sigue en él.
    if let Some(prev) = at
        .checked_sub(1)
        .and_then(|b| root.descendant_for_byte_range(b, b))
        && !node.kind().contains("comment")
        && prev.kind().contains("comment")
        && prev.end_byte() >= at
    {
        node = prev;
    }
    if !node.kind().contains("comment") {
        return node;
    }
    let line_start = text[..node.start_byte()].rfind('\n').map_or(0, |i| i + 1);
    let code = text[line_start..node.start_byte()].trim_end();
    if code.trim().is_empty() {
        return node;
    }
    let before = line_start + code.len() - 1;
    root.descendant_for_byte_range(before, before)
        .unwrap_or(node)
}

fn txt<'a>(text: &'a str, n: Node) -> &'a str {
    &text[n.byte_range()]
}

fn is_ident(s: &str) -> bool {
    let mut c = s.chars();
    c.next()
        .is_some_and(|f| f.is_ascii_alphabetic() || f == '_')
        && c.all(|x| x.is_ascii_alphanumeric() || x == '_')
}

fn line_indent(text: &str, at: usize) -> String {
    let start = text[..at].rfind('\n').map_or(0, |i| i + 1);
    text[start..at]
        .chars()
        .take_while(|c| c.is_whitespace())
        .collect()
}

/// Inicio de la línea siguiente a `end`, y si hay que añadir un salto porque el fichero acaba ahí.
fn after_line(text: &str, end: usize) -> (usize, bool) {
    // Algunos comentarios de línea incluyen su salto.
    if end > 0 && text.as_bytes()[end - 1] == b'\n' {
        return (end, false);
    }
    match text[end..].find('\n') {
        Some(i) => (end + i + 1, false),
        None => (text.len(), true),
    }
}

impl SyntaxAnalyzer for Treesitter {
    fn supports(&self, language: &str) -> bool {
        self.lang(language).is_some()
    }

    fn language_for_path(&self, path: &Path) -> Option<String> {
        let ext = path.extension()?.to_str()?;
        self.langs
            .iter()
            .find(|l| l.extensions.iter().any(|e| e == ext))
            .map(|l| l.id.clone())
    }

    fn extra_snippet_languages(&self, language: &str) -> Vec<String> {
        match language {
            "typescriptreact" => vec!["typescript".to_string()],
            _ => Vec::new(),
        }
    }

    fn container(&self, language: &str, text: &str, offset: usize) -> Context {
        let Some(i) = self.lang(language) else {
            return Context::Unknown;
        };
        let tree = self.tree(i, text);
        let root = tree.root_node();
        let at = probe(text, offset);
        let range = if text.is_empty() {
            None
        } else {
            Some(at.min(text.len() - 1)..(at + 1).min(text.len()))
        };
        let containers = ids(
            &captured(&self.langs[i].containers, root, text, range),
            "container",
        );
        // Un error de una sola línea es lo que se está escribiendo; uno de varias impide decidir.
        let mut saw_error = false;
        let mut n = Some(node_at(root, text, at));
        while let Some(x) = n {
            if x.is_error() {
                saw_error |= x.start_position().row != x.end_position().row;
            } else if containers.contains(&x.id()) {
                return if saw_error {
                    Context::Unknown
                } else {
                    Context::Known(x.kind().to_string())
                };
            }
            n = x.parent();
        }
        Context::Unknown
    }

    fn imports(&self, language: &str, text: &str, offset: usize) -> ImportScope {
        let Some(i) = self.lang(language) else {
            return ImportScope::default();
        };
        let lang = &self.langs[i];
        let tree = self.tree(i, text);
        let root = tree.root_node();
        let caps = captured(&lang.imports, root, text, None);
        let (imports, headers, scopes) = (
            ids(&caps, "import"),
            ids(&caps, "header"),
            ids(&caps, "scope"),
        );

        let mut scope = root;
        let mut n = Some(node_at(root, text, probe(text, offset)));
        while let Some(x) = n {
            if scopes.contains(&x.id()) {
                scope = x;
                break;
            }
            n = x.parent();
        }

        let mut out = ImportScope::default();
        let (mut header, mut last_import, mut prologue) = (None, None, true);
        let mut cursor = scope.walk();
        for child in scope.named_children(&mut cursor) {
            if imports.contains(&child.id()) {
                last_import = Some(child);
                out.has_imports = true;
                out.statements.push(txt(text, child).to_string());
                out.imports.extend(decompose(lang.decompose, child, text));
            } else if prologue {
                if headers.contains(&child.id()) {
                    header = Some(child);
                } else if !child.kind().contains("comment") {
                    prologue = false;
                }
            }
        }
        match last_import.or(header) {
            Some(anchor) => {
                out.indent = line_indent(text, anchor.start_byte());
                (out.insert_at, out.newline_before) = after_line(text, anchor.end_byte());
            }
            None if scope == root => {}
            None => {
                // Dentro de un módulo de Rust: tras la llave de apertura.
                let open = scope.start_byte() + 1;
                let mut c = scope.walk();
                let first = scope.named_children(&mut c).next();
                out.indent = match first {
                    Some(f) => line_indent(text, f.start_byte()),
                    None => line_indent(text, scope.start_byte()) + "    ",
                };
                if scope.start_position().row == scope.end_position().row {
                    (out.insert_at, out.newline_before) = (open, true);
                } else {
                    (out.insert_at, out.newline_before) = after_line(text, open);
                }
            }
        }
        out
    }

    fn parse_requirements(
        &self,
        language: &str,
        statement: &str,
    ) -> Result<Vec<Requirement>, String> {
        let i = self
            .lang(language)
            .ok_or_else(|| format!("{language} no tiene gramática"))?;
        let lang = &self.langs[i];
        let tree = self.parse(i, statement, None);
        let root = tree.root_node();
        if root.has_error() {
            return Err("no es una sentencia válida".to_string());
        }
        let imports = ids(&captured(&lang.imports, root, statement, None), "import");
        let mut cursor = root.walk();
        let nodes: Vec<Node> = root
            .named_children(&mut cursor)
            .filter(|c| imports.contains(&c.id()))
            .collect();
        if nodes.is_empty() {
            return Err("no es un import".to_string());
        }
        Ok(nodes
            .into_iter()
            .flat_map(|n| decompose(lang.decompose, n, statement))
            .collect())
    }

    fn tokens(&self, language: &str, text: &str) -> Vec<Token> {
        let Some(i) = self.lang(language) else {
            return Vec::new();
        };
        let tree = self.tree(i, text);
        let mut out = Vec::new();
        walk_tokens(tree.root_node(), text, &mut out);
        out
    }

    fn fragments(&self, language: &str, text: &str, min_lines: usize) -> Vec<Fragment> {
        let Some(i) = self.lang(language) else {
            return Vec::new();
        };
        let tree = self.tree(i, text);
        let mut out = Vec::new();
        hash_node(tree.root_node(), min_lines, true, &mut out);
        out
    }
}

fn walk_tokens(n: Node, text: &str, out: &mut Vec<Token>) {
    if n.is_named() && LITERALS.contains(&n.kind()) {
        out.push(Token {
            start: n.start_byte(),
            end: n.end_byte(),
            kind: TokenKind::Literal,
        });
        return;
    }
    if n.kind().contains("comment") {
        return;
    }
    if n.child_count() == 0 {
        if n.is_named() && is_ident(txt(text, n)) {
            out.push(Token {
                start: n.start_byte(),
                end: n.end_byte(),
                kind: TokenKind::Ident,
            });
        }
        return;
    }
    let mut c = n.walk();
    for child in n.children(&mut c) {
        walk_tokens(child, text, out);
    }
}

/// Huella de la estructura: tipos de nodo y tokens fijos; nombres, literales y comentarios no cuentan.
fn hash_node(n: Node, min_lines: usize, is_root: bool, out: &mut Vec<Fragment>) -> u64 {
    let mut h = DefaultHasher::new();
    n.kind_id().hash(&mut h);
    if !(n.is_named() && LITERALS.contains(&n.kind())) {
        let mut c = n.walk();
        for child in n.children(&mut c) {
            if !child.kind().contains("comment") {
                hash_node(child, min_lines, false, out).hash(&mut h);
            }
        }
    }
    let hash = h.finish();
    let (start, end) = (n.start_position().row, n.end_position().row);
    if !is_root && n.is_named() && !n.is_error() && end - start + 1 >= min_lines {
        out.push(Fragment {
            hash,
            start_line: start,
            end_line: end,
            start_byte: n.start_byte(),
            end_byte: n.end_byte(),
        });
    }
    hash
}

fn decompose(kind: Decompose, node: Node, text: &str) -> Vec<Requirement> {
    let statement = txt(text, node).trim().to_string();
    let start = node.start_byte();
    let mk = |name: String, single: String, relative: bool| Requirement {
        name,
        single,
        relative,
        statement: statement.clone(),
        start,
    };
    match kind {
        Decompose::Rust => rust_imports(node, text)
            .into_iter()
            .map(|(n, s, r)| mk(n, s, r))
            .collect(),
        Decompose::TypeScript => ts_imports(node, text)
            .into_iter()
            .map(|(n, s, r)| mk(n, s, r))
            .collect(),
        Decompose::Python => py_imports(node, text)
            .into_iter()
            .map(|(n, s, r)| mk(n, s, r))
            .collect(),
        Decompose::None => Vec::new(),
    }
}

/// `use a::{b::C, d as e, self};` → (nombre, `use ...;` de un solo nombre, relativo).
fn rust_imports(node: Node, text: &str) -> Vec<(String, String, bool)> {
    let vis = node
        .named_child(0)
        .filter(|c| c.kind() == "visibility_modifier")
        .map(|v| format!("{} ", txt(text, v)))
        .unwrap_or_default();
    let mut paths = Vec::new();
    if let Some(arg) = node.child_by_field_name("argument") {
        rust_tree(arg, "", text, &mut paths);
    }
    paths
        .into_iter()
        .map(|(path, alias)| {
            let name = alias
                .clone()
                .unwrap_or_else(|| path.rsplit("::").next().unwrap_or(&path).to_string());
            let relative = path == "self"
                || path == "super"
                || path.starts_with("self::")
                || path.starts_with("super::");
            let alias = alias.map(|a| format!(" as {a}")).unwrap_or_default();
            (name, format!("{vis}use {path}{alias};"), relative)
        })
        .collect()
}

fn rust_tree(n: Node, prefix: &str, text: &str, out: &mut Vec<(String, Option<String>)>) {
    let clean = |n: Node| {
        txt(text, n)
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
    };
    let join = |s: String| {
        if prefix.is_empty() {
            s
        } else {
            format!("{prefix}::{s}")
        }
    };
    match n.kind() {
        "use_as_clause" => {
            if let (Some(p), Some(a)) = (
                n.child_by_field_name("path"),
                n.child_by_field_name("alias"),
            ) {
                out.push((join(clean(p)), Some(txt(text, a).to_string())));
            }
        }
        "scoped_use_list" => {
            let p = n
                .child_by_field_name("path")
                .map_or(prefix.to_string(), |p| join(clean(p)));
            if let Some(list) = n.child_by_field_name("list") {
                rust_tree(list, &p, text, out);
            }
        }
        "use_list" => {
            let mut c = n.walk();
            for child in n.named_children(&mut c) {
                rust_tree(child, prefix, text, out);
            }
        }
        "self" if !prefix.is_empty() => out.push((prefix.to_string(), None)),
        k if k == "use_wildcard" || k.contains("comment") => {}
        _ => out.push((join(clean(n)), None)),
    }
}

/// `import d, { a as b } from "m";` → una entrada por nombre.
fn ts_imports(node: Node, text: &str) -> Vec<(String, String, bool)> {
    let Some(source) = node.child_by_field_name("source") else {
        return Vec::new();
    };
    let src = txt(text, source);
    let inner = src.trim_matches(['"', '\'', '`']);
    let relative = inner.starts_with('.') || inner.starts_with('/');
    let mut c = node.walk();
    let type_kw = if node
        .children(&mut c)
        .any(|ch| !ch.is_named() && ch.kind() == "type")
    {
        "type "
    } else {
        ""
    };
    let semi = if txt(text, node).trim_end().ends_with(';') {
        ";"
    } else {
        ""
    };
    let mut out = Vec::new();
    let mut c = node.walk();
    for clause in node
        .named_children(&mut c)
        .filter(|n| n.kind() == "import_clause")
    {
        let mut cc = clause.walk();
        for part in clause.named_children(&mut cc) {
            match part.kind() {
                "identifier" => {
                    let name = txt(text, part);
                    out.push((
                        name.to_string(),
                        format!("import {type_kw}{name} from {src}{semi}"),
                        relative,
                    ));
                }
                "namespace_import" => {
                    if let Some(id) = part.named_child(0) {
                        let name = txt(text, id);
                        out.push((
                            name.to_string(),
                            format!("import {type_kw}* as {name} from {src}{semi}"),
                            relative,
                        ));
                    }
                }
                "named_imports" => {
                    let mut sc = part.walk();
                    for spec in part
                        .named_children(&mut sc)
                        .filter(|s| s.kind() == "import_specifier")
                    {
                        let Some(local) = spec
                            .child_by_field_name("alias")
                            .or(spec.child_by_field_name("name"))
                        else {
                            continue;
                        };
                        out.push((
                            txt(text, local).to_string(),
                            format!("import {type_kw}{{ {} }} from {src}{semi}", txt(text, spec)),
                            relative,
                        ));
                    }
                }
                _ => {}
            }
        }
    }
    out
}

/// `import a.b as c` y `from m import x, y as z` → una entrada por nombre.
fn py_imports(node: Node, text: &str) -> Vec<(String, String, bool)> {
    let mut out = Vec::new();
    let module = match node.kind() {
        "import_statement" => None,
        "future_import_statement" => Some("__future__".to_string()),
        _ => node
            .child_by_field_name("module_name")
            .map(|m| txt(text, m).to_string()),
    };
    let relative = module.as_deref().is_some_and(|m| m.starts_with('.'));
    let mut c = node.walk();
    for name in node.children_by_field_name("name", &mut c) {
        let (imported, local) = match name.kind() {
            "aliased_import" => match (
                name.child_by_field_name("name"),
                name.child_by_field_name("alias"),
            ) {
                (Some(n), Some(a)) => (txt(text, n), Some(txt(text, a))),
                _ => continue,
            },
            _ => (txt(text, name), None),
        };
        let alias = local.map(|a| format!(" as {a}")).unwrap_or_default();
        let single = match &module {
            Some(m) => format!("from {m} import {imported}{alias}"),
            None => format!("import {imported}{alias}"),
        };
        out.push((local.unwrap_or(imported).to_string(), single, relative));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incremental_parse_matches_full_parse() {
        let t = Treesitter::new();
        let i = t.lang("rust").unwrap();
        let texts = [
            "fn a() {}\n",
            "fn a() { é }\nfn b() {}\n",
            "fn a() { è }\n// ñ\nfn b() {}\n",
            "struct S;\n",
            "",
        ];
        for w in texts.windows(2) {
            t.tree(i, w[0]);
            let incremental = t.tree(i, w[1]).root_node().to_sexp();
            assert_eq!(
                incremental,
                t.parse(i, w[1], None).root_node().to_sexp(),
                "{:?}",
                w
            );
        }
    }
}
