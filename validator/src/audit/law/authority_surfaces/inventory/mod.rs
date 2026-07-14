use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

mod files;
mod provenance;

pub(super) const DEAUTHORIZED_COMMAND_INVENTORY: &str =
    "docs/generated/observability/command-inventory.json";

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
        if rel == DEAUTHORIZED_COMMAND_INVENTORY {
            continue;
        }
        let value = crate::json_boundary::read_json(&root.join(&rel)).unwrap_or(Value::Null);
        if provenance::missing_generated_provenance(&value) {
            push(
                &mut out,
                "generated-proof-artifact-provenance-anti-fabrication",
                format!("generated_artifact_missing_provenance:{rel}"),
            );
        }
        out.extend(generated_row_provenance_failures(&rel, &value));
        out.extend(generated_product_opaque_path_failures(&rel, &value));
        if provenance::runtime_fixture_claims_artifact_truth(&value) {
            push(
                &mut out,
                "generated-proof-artifact-provenance-anti-fabrication",
                format!("runtime_normalized_fixture_used_as_artifact_truth:{rel}"),
            );
        }
        if provenance::hand_edits_allowed(&value) {
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

fn generated_row_provenance_failures(rel: &str, value: &Value) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for key in inventory_map_keys(value) {
        let Some(rows) = value.get(&key).and_then(Value::as_object) else {
            continue;
        };
        for (row_id, row) in rows {
            if !provenance::has_row_provenance(row) {
                push(
                    &mut out,
                    "generated-proof-artifact-provenance-anti-fabrication",
                    format!("generated_inventory_row_missing_provenance:{rel}:{key}:{row_id}"),
                );
            }
            if provenance::hand_edits_allowed(row) {
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
    files::generated_files(root)
}

fn push(out: &mut Vec<(String, String)>, check: &str, detail: String) {
    out.push((check.to_string(), detail));
}
