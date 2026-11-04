use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct StatsRow {
    pub id: SnippetId,
    pub uses: u64,
    pub last_use: Option<i64>,
    pub first_seen: Option<i64>,
    pub candidate: bool,
}

impl Engine {
    /// Uso de cada snippet que se aplica en el proyecto, y si es candidato a borrar.
    pub fn stats(&mut self, root: Option<&Path>) -> Result<(Vec<StatsRow>, Vec<Notice>), String> {
        let mut notices = Vec::new();
        let config = self.config(root, &mut notices);
        let languages = self.snippet_languages(root);
        let mut ids: Vec<SnippetId> = self
            .effective(root, &languages, &mut notices)
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        ids.sort_by(|a, b| (&a.language, &a.key, a.layer).cmp(&(&b.language, &b.key, b.layer)));
        let usage = self.ports.usage.stats(&ids)?;
        let now = self.ports.clock.now();
        let rows = ids
            .into_iter()
            .zip(usage)
            .map(|(id, u)| {
                // Sin usos cuenta desde el alta, para no marcar lo que se acaba de crear.
                let since = u.last_use.or(u.first_seen).unwrap_or(now);
                StatsRow {
                    candidate: now - since >= config.unused_days * DAY,
                    id,
                    uses: u.uses,
                    last_use: u.last_use,
                    first_seen: u.first_seen,
                }
            })
            .collect();
        Ok((rows, notices))
    }
}

pub fn stats_markdown(rows: &[StatsRow]) -> String {
    let mut out = String::from("# molde stats\n\n");
    if rows.is_empty() {
        out.push_str("No hay snippets.\n");
        return out;
    }
    out.push_str("| snippet | capa | lenguaje | usos | último uso | alta | ¿borrar? |\n|---|---|---|---:|---|---|---|\n");
    for r in rows {
        let layer = match r.id.layer {
            Layer::User => "usuario",
            Layer::Project => "proyecto",
        };
        out.push_str(&format!(
            "| {} | {layer} | {} | {} | {} | {} | {} |\n",
            r.id.key,
            r.id.language,
            r.uses,
            r.last_use.map_or("nunca".to_string(), date),
            r.first_seen.map_or("-".to_string(), date),
            if r.candidate { "sí" } else { "" }
        ));
    }
    let n = rows.iter().filter(|r| r.candidate).count();
    out.push_str(&format!(
        "\n{} snippets, {n} candidatos a borrar.\n",
        rows.len()
    ));
    out
}

/// AAAA-MM-DD en UTC.
pub fn date(ts: i64) -> String {
    // Algoritmo civil_from_days de Howard Hinnant.
    let z = ts.div_euclid(DAY) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    #[test]
    fn dates() {
        assert_eq!(super::date(0), "1970-01-01");
        assert_eq!(super::date(951_782_400), "2000-02-29");
        assert_eq!(super::date(1_793_750_400), "2026-11-04");
    }
}
