use crate::json_boundary;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

pub fn catalog_completeness_errors(root: &Path, rows: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    let catalog_paths = rows
        .iter()
        .filter_map(|row| row.get("path").and_then(Value::as_str))
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let catalog_ids = rows
        .iter()
        .filter_map(|row| row.get("id").and_then(Value::as_str))
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    push_duplicates(
        &mut errors,
        &catalog_paths,
        "schema-catalog duplicate paths",
    );
    push_duplicates(&mut errors, &catalog_ids, "schema-catalog duplicate ids");
    path_set_checks(root, &catalog_paths, &mut errors);
    errors
}

pub fn ref_errors(schema: &Value, allowed: &BTreeSet<String>) -> Vec<String> {
    let mut errors = Vec::new();
    visit_refs(schema, allowed, &mut errors);
    errors
}

fn path_set_checks(root: &Path, catalog_paths: &[String], errors: &mut Vec<String>) {
    let expected_paths = expected_schema_paths(root);
    let catalog_set = catalog_paths.iter().cloned().collect::<BTreeSet<_>>();
    let missing = expected_paths
        .difference(&catalog_set)
        .take(5)
        .cloned()
        .collect::<Vec<_>>();
    let extra = catalog_set
        .difference(&expected_paths)
        .take(5)
        .cloned()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        errors.push(format!("schema-catalog missing schema paths: {missing:?}"));
    }
    if !extra.is_empty() {
        errors.push(format!("schema-catalog extra schema paths: {extra:?}"));
    }
}

fn expected_schema_paths(root: &Path) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    if let Ok(entries) = std::fs::read_dir(root.join("schemas")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|p| p.to_str()) == Some("json")
                && path
                    .file_name()
                    .and_then(|p| p.to_str())
                    .is_some_and(|name| name.ends_with(".schema.json"))
                && let Some(name) = path.file_name()
            {
                out.insert(format!("schemas/{}", name.to_string_lossy()));
            }
        }
    }
    if let Ok(manifest) = json_boundary::read_json(&root.join("plugin-manifest-draft.json")) {
        for path in json_boundary::string_array(&manifest, "schemas") {
            out.insert(path);
        }
    }
    out
}

fn visit_refs(node: &Value, allowed: &BTreeSet<String>, errors: &mut Vec<String>) {
    match node {
        Value::Object(obj) => {
            if let Some(reference) = obj.get("$ref").and_then(Value::as_str) {
                let base = reference.split('#').next().unwrap_or_default();
                if !base.is_empty() && !allowed.contains(base) {
                    if base.contains(':') {
                        errors.push(format!("schema ref not in offline catalog: {reference}"));
                    } else {
                        errors.push(format!(
                            "schema ref missing from offline catalog: {reference}"
                        ));
                    }
                }
            }
            for value in obj.values() {
                visit_refs(value, allowed, errors);
            }
        }
        Value::Array(items) => {
            for value in items {
                visit_refs(value, allowed, errors);
            }
        }
        _ => {}
    }
}

fn push_duplicates(errors: &mut Vec<String>, values: &[String], label: &str) {
    let mut seen = BTreeSet::new();
    let mut dupes = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            dupes.insert(value.clone());
        }
    }
    if !dupes.is_empty() {
        errors.push(format!(
            "{label}: {:?}",
            dupes.into_iter().take(5).collect::<Vec<_>>()
        ));
    }
}
