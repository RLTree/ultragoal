use serde_json::Value;
use std::path::Path;

#[cfg(test)]
mod tests;

pub(crate) const SUCCESSOR_CATALOG_UNAVAILABLE: &str =
    "HCT-OBSERVE successor catalog unavailable/not adopted";

pub(super) fn check(root: &Path, out: &mut Vec<String>) {
    require_law_rows(root, out);
    out.push(SUCCESSOR_CATALOG_UNAVAILABLE.to_owned());
}

#[cfg(test)]
pub(crate) fn required_commands() -> &'static [&'static str] {
    &[]
}

#[cfg(test)]
pub(crate) fn required_surfaces() -> &'static [&'static str] {
    &[]
}

#[cfg(test)]
pub(crate) fn required_loop_stages() -> &'static [&'static str] {
    &[]
}

#[cfg(test)]
pub(crate) fn required_signal_classes() -> &'static [&'static str] {
    &[]
}

#[cfg(test)]
pub(crate) fn required_dimension_families() -> Vec<(
    &'static str,
    &'static str,
    &'static str,
    &'static [&'static str],
)> {
    Vec::new()
}

fn require_law_rows(root: &Path, out: &mut Vec<String>) {
    for (rel, key, code) in [
        (
            "templates/agent-standards/enforcement.json",
            "rows",
            "missing_standards_row",
        ),
        (
            "docs/source-obligation-matrix.json",
            "obligations",
            "missing_source_obligation",
        ),
        (
            "docs/foundational-law-traceability.json",
            "entries",
            "missing_foundational_trace",
        ),
    ] {
        if !has_law_id(root, rel, key) {
            out.push(format!("observability_{code}:{}", super::LAW));
        }
    }
    let valid = format!("fixtures/mandatory-law-surfaces/valid/{}.json", super::LAW);
    if !root.join(&valid).is_file() {
        out.push(format!("observability_missing_valid_fixture:{valid}"));
    }
}

fn has_law_id(root: &Path, rel: &str, key: &str) -> bool {
    super::read::json(root, rel)
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(|rows| {
            rows.iter().any(|row| {
                row.get("id")
                    .or_else(|| row.get("obligation_id"))
                    .and_then(Value::as_str)
                    == Some(super::LAW)
            })
        })
}
