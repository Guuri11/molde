//! Palabras de un identificador o de un nombre, y sus variantes de mayúsculas.

use std::ops::Range;

/// Rangos de las palabras de `s`: separa por `_`, `-`, espacios, camelCase y PascalCase.
pub fn parts(s: &str) -> Vec<Range<usize>> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for i in 0..b.len() {
        let c = b[i];
        if !c.is_ascii_alphanumeric() {
            if let Some(st) = start.take() {
                out.push(st..i);
            }
            continue;
        }
        match start {
            None => start = Some(i),
            Some(st) => {
                let prev = b[i - 1];
                let lower_to_upper =
                    (prev.is_ascii_lowercase() || prev.is_ascii_digit()) && c.is_ascii_uppercase();
                let acronym_end = prev.is_ascii_uppercase()
                    && c.is_ascii_uppercase()
                    && b.get(i + 1).is_some_and(u8::is_ascii_lowercase);
                if lower_to_upper || acronym_end {
                    out.push(st..i);
                    start = Some(i);
                }
            }
        }
    }
    if let Some(st) = start {
        out.push(st..b.len());
    }
    out
}

/// Palabras en minúsculas.
pub fn words(s: &str) -> Vec<String> {
    parts(s).into_iter().map(|r| s[r].to_lowercase()).collect()
}

/// Plural inglés regular de la última palabra. Los irregulares se pasan con `plural=`.
pub fn pluralize(words: &[String]) -> Vec<String> {
    let mut w = words.to_vec();
    if let Some(last) = w.last_mut() {
        let consonant_y =
            last.ends_with('y') && !["ay", "ey", "oy", "uy"].iter().any(|e| last.ends_with(e));
        *last = if consonant_y {
            format!("{}ies", &last[..last.len() - 1])
        } else if ["s", "x", "z", "ch", "sh"]
            .iter()
            .any(|e| last.ends_with(e))
        {
            format!("{last}es")
        } else {
            format!("{last}s")
        };
    }
    w
}

fn capitalize(w: &str) -> String {
    let mut c = w.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

/// Variantes de un nombre: snake, kebab, pascal, camel, screaming, title y lower.
pub fn forms(words: &[String]) -> [(&'static str, String); 7] {
    let caps: Vec<String> = words.iter().map(|w| capitalize(w)).collect();
    let camel = words
        .first()
        .map(|f| f.clone() + &caps[1..].concat())
        .unwrap_or_default();
    [
        ("snake", words.join("_")),
        ("kebab", words.join("-")),
        ("pascal", caps.concat()),
        ("camel", camel),
        ("screaming", words.join("_").to_uppercase()),
        ("title", caps.join(" ")),
        ("lower", words.join(" ")),
    ]
}

/// Pares (origen, destino) de todas las variantes, de la más larga a la más corta,
/// para que el plural se sustituya antes que el singular.
pub fn pairs(
    from: &[String],
    from_plural: &[String],
    to: &[String],
    to_plural: &[String],
) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for (f, t) in [(from_plural, to_plural), (from, to)] {
        for ((_, a), (_, b)) in forms(f).into_iter().zip(forms(t)) {
            if !out.iter().any(|(x, _)| *x == a) {
                out.push((a, b));
            }
        }
    }
    out.sort_by_key(|p| std::cmp::Reverse(p.0.len()));
    out
}

/// Sustituye en una sola pasada, probando primero la variante más larga.
pub fn substitute(text: &str, pairs: &[(String, String)]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    'outer: while i < text.len() {
        for (from, to) in pairs {
            if !from.is_empty() && text[i..].starts_with(from.as_str()) {
                out.push_str(to);
                i += from.len();
                continue 'outer;
            }
        }
        let ch = text[i..].chars().next().unwrap_or_default();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Rellena `{snake}`, `{pascal}`... y sus versiones en plural `{snake_plural}`...
pub fn render(template: &str, words: &[String], plural: &[String]) -> String {
    let mut out = template.to_string();
    for ((name, s), (_, p)) in forms(words).into_iter().zip(forms(plural)) {
        out = out
            .replace(&format!("{{{name}_plural}}"), &p)
            .replace(&format!("{{{name}}}"), &s);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_and_substitutes() {
        assert_eq!(
            words("getHTTPResponse_v2"),
            ["get", "http", "response", "v2"]
        );
        assert_eq!(words("line-item"), ["line", "item"]);
        let w = |s: &str| words(s);
        let p = pairs(
            &w("sample widget"),
            &pluralize(&w("sample widget")),
            &w("line-item"),
            &pluralize(&w("line-item")),
        );
        assert_eq!(
            substitute(
                "sample_widgets SampleWidget sampleWidget SAMPLE_WIDGET Sample Widget sample-widget",
                &p
            ),
            "line_items LineItem lineItem LINE_ITEM Line Item line-item"
        );
        assert_eq!(pluralize(&w("category")), ["categories"]);
        assert_eq!(
            render(
                "src/{snake}/{pascal_plural}",
                &w("line item"),
                &w("line items")
            ),
            "src/line_item/LineItems"
        );
    }
}
