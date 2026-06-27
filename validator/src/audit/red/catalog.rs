use crate::json_boundary;
use crate::schema_catalog::SchemaStore;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub(crate) fn check(
    root: &Path,
    store: &SchemaStore,
    failures: &mut BTreeMap<String, Vec<String>>,
) {
    let rows = match json_boundary::read_json(&root.join("templates/RED_FIXTURES.json")) {
        Ok(value) => value,
        Err(err) => {
            push(failures, format!("red catalog load failed: {err}"));
            return;
        }
    };
    let Some(items) = rows.as_array() else {
        push(failures, "red catalog must be an array");
        return;
    };
    let red_files = std::fs::read_dir(root.join("fixtures/red"))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("json"))
        .count();
    let ids = items
        .iter()
        .filter_map(|row| row.get("id").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    if items.len() != red_files || ids.len() != items.len() {
        push(failures, "red catalog count or id uniqueness mismatch");
    }
    crate::audit::red::identity::check(root, store, &ids, failures);
    for row in items {
        catalog_row_check(root, row, failures);
    }
}

fn catalog_row_check(root: &Path, row: &Value, failures: &mut BTreeMap<String, Vec<String>>) {
    let rel = row.get("packet_path").and_then(Value::as_str).unwrap_or("");
    if let Some(error) = crate::package::inventory::package_path_error(root, rel) {
        push(
            failures,
            format!("red catalog invalid path: {rel}: {error}"),
        );
        return;
    }
    let packet = json_boundary::read_json(&root.join(rel)).ok();
    digest_check(root, row, rel, failures);
    if packet
        .as_ref()
        .and_then(|value| value.get("expected_failure"))
        != row.get("expected_failure")
    {
        push(
            failures,
            format!("red catalog expected_failure drift: {rel}"),
        );
    }
}

fn digest_check(root: &Path, row: &Value, rel: &str, failures: &mut BTreeMap<String, Vec<String>>) {
    let expected = row
        .get("packet_digest")
        .and_then(Value::as_str)
        .unwrap_or("");
    let actual =
        crate::digest::file(&root.join(rel)).unwrap_or_else(|_| crate::digest::ZERO.to_string());
    if actual != expected {
        push(failures, format!("red catalog digest mismatch: {rel}"));
    }
}

fn push(failures: &mut BTreeMap<String, Vec<String>>, detail: impl Into<String>) {
    failures
        .entry("red-fixture-coverage".to_string())
        .or_default()
        .push(detail.into());
}
