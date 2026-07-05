use super::{AuthoritySurfaceInventoryRow, authority_row, collect_files};
use std::collections::BTreeSet;
use std::path::Path;

pub(super) fn rows(root: &Path, inventory: &BTreeSet<String>) -> Vec<AuthoritySurfaceInventoryRow> {
    let mut rows = source_rows(root, inventory);
    rows.extend(claim_guard_rows(root, inventory));
    rows
}

fn source_rows(root: &Path, inventory: &BTreeSet<String>) -> Vec<AuthoritySurfaceInventoryRow> {
    let mut out = Vec::new();
    collect_files(root, &root.join("validator/src"), &mut out);
    out.into_iter()
        .filter(|path| path.ends_with(".rs"))
        .map(|path| authority_row("source", &path, true, root, inventory))
        .collect()
}

fn claim_guard_rows(
    root: &Path,
    inventory: &BTreeSet<String>,
) -> Vec<AuthoritySurfaceInventoryRow> {
    let mut out = Vec::new();
    collect_files(root, &root.join("validator/src"), &mut out);
    out.into_iter()
        .filter(|path| path.ends_with(".rs"))
        .filter(|path| claim_guard_source(root, path))
        .map(|path| authority_row("claim_guard", &path, true, root, inventory))
        .collect()
}

fn claim_guard_source(root: &Path, path: &str) -> bool {
    let text = std::fs::read_to_string(root.join(path)).unwrap_or_default();
    [
        "claim_ceiling",
        "claim_impact",
        "blocked_claims",
        "supported_claims",
        "unsupported_claims",
        "withheld_or_blocked",
        "claim_guard",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}
