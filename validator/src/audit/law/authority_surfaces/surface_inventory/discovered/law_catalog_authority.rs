use super::{AuthoritySurfaceInventoryRow, authority_row, collect_files};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

const FOUNDATIONAL_LAW_RED_FIXTURE_PREFIXES: &[&str] = &[
    "authority-source-binding",
    "typed-records-over-prose",
    "generated-proof-artifact-provenance-anti-fabrication",
    "distinct-proof-surfaces-claim-ceilings",
    "total-authority-types-impossible-state-elimination",
    "raw-private-artifact-handling-category-only-evidence",
    "validator-theater-miswire-resistance",
    "namespace-progressive-disclosure",
    "semantic-domain-type-naming",
];

pub(super) fn rows(root: &Path, inventory: &BTreeSet<String>) -> Vec<AuthoritySurfaceInventoryRow> {
    let mut rows = Vec::new();
    rows.extend(schema_rows(root, inventory));
    rows.extend(valid_fixture_rows(root, inventory));
    rows.extend(red_fixture_rows(root, inventory));
    rows.extend(standards_rows(root, inventory));
    rows.extend(source_obligation_rows(root, inventory));
    rows.extend(foundational_trace_rows(root, inventory));
    rows
}

fn schema_rows(root: &Path, inventory: &BTreeSet<String>) -> Vec<AuthoritySurfaceInventoryRow> {
    let mut out = Vec::new();
    collect_files(root, &root.join("schemas"), &mut out);
    out.into_iter()
        .filter(|path| path.ends_with(".json"))
        .map(|path| authority_row("schema", &path, true, root, inventory))
        .collect()
}

fn valid_fixture_rows(
    root: &Path,
    inventory: &BTreeSet<String>,
) -> Vec<AuthoritySurfaceInventoryRow> {
    let mut out = Vec::new();
    collect_files(
        root,
        &root.join("fixtures/mandatory-law-surfaces/valid"),
        &mut out,
    );
    out.into_iter()
        .filter(|path| path.ends_with(".json"))
        .map(|path| authority_row("valid_fixture", &path, true, root, inventory))
        .collect()
}

fn red_fixture_rows(
    root: &Path,
    inventory: &BTreeSet<String>,
) -> Vec<AuthoritySurfaceInventoryRow> {
    let mut rows = all_red_fixture_rows(root, inventory);
    rows.extend(foundational_red_fixture_rows(root, inventory));
    rows
}

fn all_red_fixture_rows(
    root: &Path,
    inventory: &BTreeSet<String>,
) -> Vec<AuthoritySurfaceInventoryRow> {
    let mut out = Vec::new();
    collect_files(root, &root.join("fixtures/red"), &mut out);
    out.into_iter()
        .filter(|path| path.ends_with(".json"))
        .map(|path| authority_row("red_fixture", &path, true, root, inventory))
        .collect()
}

fn foundational_red_fixture_rows(
    root: &Path,
    inventory: &BTreeSet<String>,
) -> Vec<AuthoritySurfaceInventoryRow> {
    let catalog = crate::json_boundary::read_json(&root.join("templates/RED_FIXTURES.json"))
        .unwrap_or(Value::Null);
    catalog
        .as_array()
        .into_iter()
        .flat_map(|rows| rows.iter())
        .filter_map(|row| {
            let id = row.get("id").and_then(Value::as_str)?;
            if !FOUNDATIONAL_LAW_RED_FIXTURE_PREFIXES
                .iter()
                .any(|prefix| id.starts_with(prefix))
            {
                return None;
            }
            row.get("packet_path")
                .and_then(Value::as_str)
                .map(|path| authority_row("red_fixture", path, true, root, inventory))
        })
        .collect()
}

fn standards_rows(root: &Path, inventory: &BTreeSet<String>) -> Vec<AuthoritySurfaceInventoryRow> {
    let mut out = Vec::new();
    collect_files(root, &root.join("templates/agent-standards"), &mut out);
    out.into_iter()
        .map(|path| authority_row("standards", &path, true, root, inventory))
        .collect()
}

fn source_obligation_rows(
    root: &Path,
    inventory: &BTreeSet<String>,
) -> Vec<AuthoritySurfaceInventoryRow> {
    let mut out = Vec::new();
    collect_files(root, &root.join("docs"), &mut out);
    out.into_iter()
        .filter(|path| {
            path.starts_with("docs/source-obligation-")
                && (path.ends_with(".json") || path.ends_with(".md"))
        })
        .map(|path| authority_row("source_obligation", &path, true, root, inventory))
        .collect()
}

fn foundational_trace_rows(
    root: &Path,
    inventory: &BTreeSet<String>,
) -> Vec<AuthoritySurfaceInventoryRow> {
    let mut out = Vec::new();
    collect_files(root, &root.join("docs"), &mut out);
    out.into_iter()
        .filter(|path| {
            path.starts_with("docs/foundational-law-traceability")
                && (path.ends_with(".json") || path.ends_with(".md"))
        })
        .map(|path| authority_row("foundational_trace", &path, true, root, inventory))
        .collect()
}
