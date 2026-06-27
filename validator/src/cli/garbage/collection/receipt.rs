use crate::cli::garbage::collection::types::GC_RECEIPT_SCHEMA;
use serde_json::Value;

pub(crate) fn surface_value_failures(value: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if value.get("schema").and_then(Value::as_str) != Some(GC_RECEIPT_SCHEMA) {
        out.push("workspace_gc_receipt_wrong_schema".to_string());
    }
    for ptr in [
        "/issuer/tool",
        "/command/name",
        "/law_ids",
        "/digests/candidate",
        "/artifact_classification/classes",
        "/protected_set/protected_artifacts",
        "/deletion_plan/plan_digest",
        "/post_verify/protected_artifacts_preserved",
        "/claim_ceiling",
    ] {
        if value.pointer(ptr).is_none() {
            out.push(format!("workspace_gc_receipt_missing:{ptr}"));
        }
    }
    let law_found = value
        .get("law_ids")
        .and_then(Value::as_array)
        .is_some_and(|rows| {
            rows.iter()
                .any(|row| row.as_str() == Some("workspace-artifact-cache-garbage-collection"))
        });
    if !law_found {
        out.push("workspace_gc_receipt_missing_law_id".to_string());
    }
    out
}
