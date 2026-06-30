use serde_json::Value;
use std::path::Path;

mod fitting;
mod operating;
mod proof;
mod surfaces;

#[cfg(test)]
mod tests;

pub(super) fn check(root: &Path, out: &mut Vec<String>) {
    require_law_rows(root, out);
    require_command_inventory(root, out);
}

#[cfg(test)]
pub(crate) fn required_commands() -> &'static [&'static str] {
    fitting::REQUIRED_COMMANDS
}

#[cfg(test)]
pub(crate) fn required_surfaces() -> &'static [&'static str] {
    surfaces::REQUIRED_SURFACES
}

#[cfg(test)]
pub(crate) fn required_loop_stages() -> &'static [&'static str] {
    operating::REQUIRED_LOOP_STAGES
}

#[cfg(test)]
pub(crate) fn required_signal_classes() -> &'static [&'static str] {
    operating::REQUIRED_SIGNAL_CLASSES
}

pub(crate) fn fitting_failures(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    require_command_inventory(root, &mut out);
    out
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

fn require_command_inventory(root: &Path, out: &mut Vec<String>) {
    let value = super::read::json(root, "docs/generated/observability/command-inventory.json");
    fitting::check(root, &value, out);
    surfaces::check(root, &value, out);
    operating::check(root, &value, out);
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
