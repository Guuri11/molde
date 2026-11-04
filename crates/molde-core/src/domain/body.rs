//! Sintaxis de cuerpo de VSCode: escapar texto literal y expandir con los valores por defecto.

use std::collections::HashMap;

/// Escapa `\`, `$` y `}` para que el texto se inserte tal cual.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(c, '\\' | '$' | '}') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Texto que inserta el editor si se aceptan todos los placeholders sin cambiarlos.
/// Las variables desconocidas se insertan con su nombre, como hace VSCode.
pub fn expand(body: &str) -> Result<String, String> {
    let nodes = Parser {
        s: body.as_bytes(),
        i: 0,
    }
    .parse(false)?;
    let mut defaults = HashMap::new();
    collect(&nodes, &mut defaults);
    let mut out = Vec::new();
    render(&nodes, &defaults, &mut out, 0);
    String::from_utf8(out).map_err(|e| e.to_string())
}

enum Node {
    Text(Vec<u8>),
    Tab(u32, Option<Vec<Node>>),
    Var(Vec<u8>, Option<Vec<Node>>),
}

// Trabaja en bytes: los caracteres especiales son ASCII y nunca aparecen dentro de un carácter UTF-8.
struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn parse(&mut self, nested: bool) -> Result<Vec<Node>, String> {
        let mut out = Vec::new();
        let mut text = Vec::new();
        while let Some(&c) = self.s.get(self.i) {
            match c {
                b'\\' if matches!(self.s.get(self.i + 1), Some(b'$' | b'}' | b'\\')) => {
                    text.push(self.s[self.i + 1]);
                    self.i += 2;
                }
                b'}' if nested => break,
                b'$' => match self.dollar()? {
                    Some(node) => {
                        if !text.is_empty() {
                            out.push(Node::Text(std::mem::take(&mut text)));
                        }
                        out.push(node);
                    }
                    None => {
                        text.push(c);
                        self.i += 1;
                    }
                },
                _ => {
                    text.push(c);
                    self.i += 1;
                }
            }
        }
        if !text.is_empty() {
            out.push(Node::Text(text));
        }
        Ok(out)
    }

    fn int(&mut self) -> Option<u32> {
        let start = self.i;
        while self.s.get(self.i).is_some_and(u8::is_ascii_digit) {
            self.i += 1;
        }
        std::str::from_utf8(&self.s[start..self.i])
            .ok()?
            .parse()
            .ok()
    }

    fn name(&mut self) -> Option<Vec<u8>> {
        let start = self.i;
        if !self
            .s
            .get(self.i)
            .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')
        {
            return None;
        }
        while self
            .s
            .get(self.i)
            .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
        {
            self.i += 1;
        }
        Some(self.s[start..self.i].to_vec())
    }

    /// `$1`, `${1}`, `${1:...}`, `${1|a,b|}`, `$VAR`, `${VAR:...}` o una transformación.
    /// None si el `$` es texto normal.
    fn dollar(&mut self) -> Result<Option<Node>, String> {
        let start = self.i;
        self.i += 1;
        if let Some(n) = self.int() {
            return Ok(Some(Node::Tab(n, None)));
        }
        if let Some(v) = self.name() {
            return Ok(Some(Node::Var(v, None)));
        }
        if self.s.get(self.i) != Some(&b'{') {
            self.i = start;
            return Ok(None);
        }
        self.i += 1;
        let tab = self.int();
        let var = if tab.is_none() { self.name() } else { None };
        if tab.is_none() && var.is_none() {
            self.i = start;
            return Ok(None);
        }
        let unclosed = || format!("placeholder sin cerrar en la posición {start}");
        let default = match self.s.get(self.i) {
            Some(b'}') => None,
            Some(b':') => {
                self.i += 1;
                Some(self.parse(true)?)
            }
            Some(b'|') if tab.is_some() => {
                self.i += 1;
                Some(vec![Node::Text(self.choice().ok_or_else(unclosed)?)])
            }
            Some(b'/') => {
                self.skip_transform().ok_or_else(unclosed)?;
                None
            }
            _ => return Err(unclosed()),
        };
        if self.s.get(self.i) != Some(&b'}') {
            return Err(unclosed());
        }
        self.i += 1;
        Ok(Some(match (tab, var) {
            (Some(n), _) => Node::Tab(n, default),
            (None, v) => Node::Var(v.unwrap_or_default(), default),
        }))
    }

    /// Opciones de `${1|a,b|}`: devuelve la primera y deja el cursor en el `}`.
    fn choice(&mut self) -> Option<Vec<u8>> {
        let mut first = Vec::new();
        let mut in_first = true;
        loop {
            match *self.s.get(self.i)? {
                b'\\' => {
                    let c = *self.s.get(self.i + 1)?;
                    if in_first {
                        first.push(c);
                    }
                    self.i += 2;
                }
                b',' => {
                    in_first = false;
                    self.i += 1;
                }
                b'|' if self.s.get(self.i + 1) == Some(&b'}') => {
                    self.i += 1;
                    return Some(first);
                }
                c => {
                    if in_first {
                        first.push(c);
                    }
                    self.i += 1;
                }
            }
        }
    }

    /// Salta `/regex/formato/opciones` y deja el cursor en el `}` que lo cierra.
    fn skip_transform(&mut self) -> Option<()> {
        let mut depth = 0;
        loop {
            match *self.s.get(self.i)? {
                b'\\' => self.i += 2,
                b'$' if self.s.get(self.i + 1) == Some(&b'{') => {
                    depth += 1;
                    self.i += 2;
                }
                b'}' if depth == 0 => return Some(()),
                b'}' => {
                    depth -= 1;
                    self.i += 1;
                }
                _ => self.i += 1,
            }
        }
    }
}

/// El primer valor por defecto de cada placeholder: los demás con el mismo número lo reflejan.
fn collect<'a>(nodes: &'a [Node], defaults: &mut HashMap<u32, &'a [Node]>) {
    for n in nodes {
        match n {
            Node::Tab(i, Some(d)) => {
                defaults.entry(*i).or_insert(d);
                collect(d, defaults);
            }
            Node::Var(_, Some(d)) => collect(d, defaults),
            _ => {}
        }
    }
}

fn render(nodes: &[Node], defaults: &HashMap<u32, &[Node]>, out: &mut Vec<u8>, depth: usize) {
    // Un placeholder que se refleja dentro de su propio valor no debe recursar sin fin.
    if depth > 16 {
        return;
    }
    for n in nodes {
        match n {
            Node::Text(t) => out.extend_from_slice(t),
            Node::Tab(_, Some(d)) | Node::Var(_, Some(d)) => render(d, defaults, out, depth + 1),
            Node::Tab(i, None) => {
                if let Some(d) = defaults.get(i) {
                    render(d, defaults, out, depth + 1);
                }
            }
            // Las variables de VSCode (TM_FILENAME...) van en mayúsculas; el resto se inserta por su nombre.
            Node::Var(name, None) => {
                if !name
                    .iter()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == b'_')
                {
                    out.extend_from_slice(name);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_defaults_and_mirrors() {
        assert_eq!(
            expand("fn get_${1:invoice}() -> ${2:Invoice} { ${1} }$0").unwrap(),
            "fn get_invoice() -> Invoice { invoice }"
        );
        assert_eq!(
            expand("${1|a,b|} \\$x \\} ${TM_FILENAME} ${base}").unwrap(),
            "a $x }  base"
        );
        assert_eq!(
            expand(&escape("`${base}/x/${id}` \\ $1")).unwrap(),
            "`${base}/x/${id}` \\ $1"
        );
        assert!(expand("${1:sin cerrar").is_err());
    }
}
