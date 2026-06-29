pub(crate) mod checks;
use std::collections::BTreeMap;
use std::path::Path;

pub fn parse(
    root: &Path,
    rel: &str,
    header: &[&str],
) -> Result<Vec<BTreeMap<String, String>>, String> {
    let text = std::fs::read_to_string(root.join(rel))
        .map_err(|err| format!("agent_standards_tsv_read_failed:{rel}:{err}"))?;
    let mut lines = text.lines();
    let got = lines
        .next()
        .map(|line| line.split('\t').collect::<Vec<_>>())
        .unwrap_or_default();
    if got != header {
        return Err(format!("agent_standards_tsv_bad_header:{rel}"));
    }
    let mut out = Vec::new();
    for line in lines.filter(|line| !line.trim().is_empty()) {
        let cells = line.split('\t').collect::<Vec<_>>();
        if cells.len() != header.len() {
            return Err(format!("agent_standards_tsv_bad_row:{rel}"));
        }
        out.push(
            header
                .iter()
                .zip(cells)
                .map(|(key, value)| ((*key).to_string(), value.to_string()))
                .collect(),
        );
    }
    Ok(out)
}

pub fn field(row: &BTreeMap<String, String>, key: &str) -> String {
    row.get(key).cloned().unwrap_or_default()
}
