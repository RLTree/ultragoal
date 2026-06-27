use crate::cli::rust::types::RUST_RECEIPT_SCHEMA;
use serde_json::Value;

pub(crate) fn surface_value_failures(value: &Value, expected_law: &str) -> Vec<String> {
    let mut out = Vec::new();
    if value.get("schema").and_then(Value::as_str) != Some(RUST_RECEIPT_SCHEMA) {
        out.push("rust_devx_receipt_wrong_schema".to_string());
    }
    for ptr in [
        "/issuer/tool",
        "/command/name",
        "/law_ids",
        "/digests/candidate",
        "/digests/cargo_lock",
        "/toolchain/rustc_version",
        "/cache/cache_mode",
        "/resource_discipline/bounded_resources",
        "/tool_observations/probes",
        "/observation_failures",
        "/staleness_policy/invalidates_on",
        "/claim_ceiling",
    ] {
        if value.pointer(ptr).is_none() {
            out.push(format!("rust_devx_receipt_missing:{ptr}"));
        }
    }
    let law_found = value
        .get("law_ids")
        .and_then(Value::as_array)
        .is_some_and(|rows| rows.iter().any(|row| row.as_str() == Some(expected_law)));
    if !law_found {
        out.push(format!("rust_devx_receipt_missing_law_id:{expected_law}"));
    }
    if value.get("status").and_then(Value::as_str) == Some("pass")
        && value.get("claim_ceiling").and_then(Value::as_str) != Some("rust_devx_observation_bound")
    {
        out.push("rust_devx_pass_without_observation_bound_ceiling".to_string());
    }
    if value.get("status").and_then(Value::as_str) == Some("pass")
        && value
            .get("observation_failures")
            .and_then(Value::as_array)
            .is_some_and(|failures| !failures.is_empty())
    {
        out.push("rust_devx_pass_with_observation_failures".to_string());
    }
    if value
        .pointer("/tool_observations/raw_output_is_authority")
        .and_then(Value::as_bool)
        != Some(false)
    {
        out.push("rust_devx_raw_output_marked_authority".to_string());
    }
    out
}
