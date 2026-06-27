use crate::schema_catalog::{self, SchemaStore};
use serde_json::Value;
use std::collections::BTreeSet;

pub(crate) fn errors(store: &SchemaStore, packet: &Value, bad: &Value) -> Vec<String> {
    if packet
        .pointer("/materialization/expected_validation_layer")
        .and_then(Value::as_str)
        == Some("schema")
    {
        return schema_catalog::schema_errors(store, "fixture-bundle.schema.json", bad);
    }
    let Some(roots) = touched_roots(packet) else {
        return schema_catalog::schema_errors(store, "fixture-bundle.schema.json", bad);
    };
    selective_errors(store, bad, &roots)
}

fn touched_roots(packet: &Value) -> Option<BTreeSet<String>> {
    let mut roots = BTreeSet::new();
    for op in packet.get("json_patch")?.as_array()? {
        let path = op.get("path")?.as_str()?;
        let root = path.trim_start_matches('/').split('/').next().unwrap_or("");
        if root.is_empty() {
            return None;
        }
        roots.insert(root.to_string());
    }
    Some(roots)
}

fn selective_errors(store: &SchemaStore, bad: &Value, roots: &BTreeSet<String>) -> Vec<String> {
    let mut errors = Vec::new();
    for root in roots {
        match root.as_str() {
            "completion_manifest" => validate_child(
                store,
                bad,
                root,
                "completion-manifest.schema.json",
                &mut errors,
            ),
            "lane_registry" => {
                validate_child(store, bad, root, "lane-registry.schema.json", &mut errors)
            }
            "ready_for_merge" => {
                validate_child(store, bad, root, "ready-for-merge.schema.json", &mut errors)
            }
            "verification_backlog" => validate_child(
                store,
                bad,
                root,
                "verification-backlog.schema.json",
                &mut errors,
            ),
            "plugin_manifest" => {
                validate_child(store, bad, root, "plugin-manifest.schema.json", &mut errors)
            }
            "automation_tick_receipt" => validate_child(
                store,
                bad,
                root,
                "automation-tick-receipt.schema.json",
                &mut errors,
            ),
            "validator_receipt" => validate_child(
                store,
                bad,
                root,
                "validator-receipt.schema.json",
                &mut errors,
            ),
            "ready_for_merge_receipts" => validate_ready_receipts(store, bad, &mut errors),
            "amendments" => validate_amendments(store, bad, &mut errors),
            _ => errors.extend(schema_catalog::schema_errors(
                store,
                "fixture-bundle.schema.json",
                bad,
            )),
        }
    }
    errors
}

fn validate_child(
    store: &SchemaStore,
    bad: &Value,
    key: &str,
    schema: &str,
    errors: &mut Vec<String>,
) {
    errors.extend(schema_catalog::schema_errors(store, schema, &bad[key]));
}

fn validate_ready_receipts(store: &SchemaStore, bad: &Value, errors: &mut Vec<String>) {
    for value in bad
        .get("ready_for_merge_receipts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        errors.extend(schema_catalog::schema_errors(
            store,
            "ready-for-merge.schema.json",
            value,
        ));
    }
}

fn validate_amendments(store: &SchemaStore, bad: &Value, errors: &mut Vec<String>) {
    for value in bad
        .get("amendments")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        errors.extend(schema_catalog::schema_errors(
            store,
            "contract-amendment.schema.json",
            value,
        ));
    }
}
