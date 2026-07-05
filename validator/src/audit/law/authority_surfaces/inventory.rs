use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

pub(super) fn generated_failures(
    root: &Path,
    inventory: &BTreeSet<String>,
) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for rel in inventory {
        if crate::package::inventory::builder_contract_resource_path(rel) {
            push(
                &mut out,
                "generated-proof-artifact-provenance-anti-fabrication",
                format!("builder_contract_file_in_package_evidence:{rel}"),
            );
        }
    }
    for rel in generated_files(root) {
        let value = crate::json_boundary::read_json(&root.join(&rel)).unwrap_or(Value::Null);
        if missing_generated_provenance(&value) {
            push(
                &mut out,
                "generated-proof-artifact-provenance-anti-fabrication",
                format!("generated_artifact_missing_provenance:{rel}"),
            );
        }
        out.extend(generated_row_provenance_failures(&rel, &value));
        out.extend(generated_product_opaque_path_failures(&rel, &value));
        if runtime_fixture_claims_artifact_truth(&value) {
            push(
                &mut out,
                "generated-proof-artifact-provenance-anti-fabrication",
                format!("runtime_normalized_fixture_used_as_artifact_truth:{rel}"),
            );
        }
        if hand_edits_allowed(&value) {
            push(
                &mut out,
                "generated-proof-artifact-provenance-anti-fabrication",
                format!("generated_artifact_hand_edit_allowed:{rel}"),
            );
        }
        if !inventory.contains(&rel) {
            push(
                &mut out,
                "generated-proof-artifact-provenance-anti-fabrication",
                format!("generated_artifact_not_in_package_inventory:{rel}"),
            );
        }
    }
    out
}

fn missing_generated_provenance(value: &Value) -> bool {
    if value
        .get("generated_from")
        .and_then(Value::as_str)
        .is_some()
        || value.pointer("/provenance/generated_from").is_some()
        || value.get("source_spec").is_some()
    {
        return false;
    }
    let provenance = value.get("provenance").unwrap_or(&Value::Null);
    !(text(provenance, "generated_artifact_type").is_some()
        && text(provenance, "validator_run_id").is_some()
        && text(provenance, "input_manifest_digest").is_some()
        && provenance.pointer("/validator_receipt/path").is_some()
        && provenance.pointer("/validator_receipt/digest").is_some())
}

fn runtime_fixture_claims_artifact_truth(value: &Value) -> bool {
    value
        .pointer("/runtime_normalized_fixture/artifact_truth")
        .and_then(Value::as_bool)
        == Some(true)
        || value
            .get("runtime_normalized_fixture_artifact_truth")
            .and_then(Value::as_bool)
            == Some(true)
}

fn hand_edits_allowed(value: &Value) -> bool {
    value.get("hand_edited").and_then(Value::as_bool) == Some(true)
        || value.get("manual_edit").and_then(Value::as_bool) == Some(true)
        || value.get("manual_edits_allowed").and_then(Value::as_bool) == Some(true)
}

fn generated_row_provenance_failures(rel: &str, value: &Value) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for key in inventory_map_keys(value) {
        let Some(rows) = value.get(&key).and_then(Value::as_object) else {
            continue;
        };
        for (row_id, row) in rows {
            if !has_row_provenance(row) {
                push(
                    &mut out,
                    "generated-proof-artifact-provenance-anti-fabrication",
                    format!("generated_inventory_row_missing_provenance:{rel}:{key}:{row_id}"),
                );
            }
            if hand_edits_allowed(row) {
                push(
                    &mut out,
                    "generated-proof-artifact-provenance-anti-fabrication",
                    format!("generated_inventory_row_hand_edit_allowed:{rel}:{key}:{row_id}"),
                );
            }
        }
    }
    out
}

const INVENTORY_MAP_KEYS: &[&str] = &[
    "command_observability_inventory",
    "validator_check_inventory",
    "receipt_proof_inventory",
    "fixture_report_inventory",
    "package_plugin_setup_retrofit_inventory",
    "operating_loop_inventory",
    "signal_inventory",
    "long_running_path_inventory",
    "external_live_path_inventory",
    "claim_guard_inventory",
    "surface_inventory",
];

fn inventory_map_keys(value: &Value) -> BTreeSet<String> {
    let mut keys = INVENTORY_MAP_KEYS
        .iter()
        .map(|key| (*key).to_string())
        .collect::<BTreeSet<_>>();
    if let Some(object) = value.as_object() {
        keys.extend(
            object
                .iter()
                .filter(|(key, row)| key.ends_with("_inventory") && row.is_object())
                .map(|(key, _)| key.to_string()),
        );
    }
    keys
}

fn has_row_provenance(row: &Value) -> bool {
    let has_owner_surface = row
        .get("current_owner_surface")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .is_some()
        || row
            .get("owner_surface")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .is_some();
    let has_source_binding = row.get("source_spec").is_some()
        || row
            .get("validator_check_id")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .is_some();
    has_owner_surface && has_source_binding
}

fn generated_product_opaque_path_failures(rel: &str, value: &Value) -> Vec<(String, String)> {
    let mut out = Vec::new();
    collect_product_opaque_path_failures(rel, "$", value, &mut out);
    out
}

fn collect_product_opaque_path_failures(
    rel: &str,
    pointer: &str,
    value: &Value,
    out: &mut Vec<(String, String)>,
) {
    match value {
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                collect_product_opaque_path_failures(rel, &format!("{pointer}/{index}"), item, out);
            }
        }
        Value::Object(map) => {
            for (key, item) in map {
                collect_product_opaque_path_failures(rel, &format!("{pointer}/{key}"), item, out);
            }
        }
        Value::String(text) if looks_like_repo_or_artifact_path(text) => {
            if let Some(label) =
                crate::audit::namespace::source::path_labels::product_opaque_goal_work_string_label(
                    text,
                )
            {
                push(
                    out,
                    "generated-proof-artifact-provenance-anti-fabrication",
                    format!(
                        "generated_artifact_product_opaque_path_segment:{rel}:{pointer}:label={label}"
                    ),
                );
            }
        }
        _ => {}
    }
}

fn looks_like_repo_or_artifact_path(text: &str) -> bool {
    text.contains('/')
        || text.ends_with(".json")
        || text.ends_with(".jsonl")
        || text.ends_with(".rs")
        || text.ends_with(".md")
}

fn generated_files(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    collect_json_files(root, &root.join("docs/generated"), &mut out);
    collect_json_files(root, &root.join("examples/generated"), &mut out);
    out
}

fn collect_json_files(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_json_files(root, &path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("json")
            && let Ok(rel) = path.strip_prefix(root)
        {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

fn push(out: &mut Vec<(String, String)>, check: &str, detail: String) {
    out.push((check.to_string(), detail));
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
}
