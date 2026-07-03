use serde_json::Value;
use std::path::Path;

const FIT_REPO_RECEIPT: &str = "validation_artifacts/harness/fit-repo-receipt.json";

pub(crate) fn failures(root: &Path, value: &Value) -> Vec<String> {
    failures_with_digest(root, value, crate::package::inventory::package_digest(root))
}

pub(crate) fn failures_with_candidate(
    root: &Path,
    value: &Value,
    target_digest: &str,
) -> Vec<String> {
    failures_with_digest(root, value, Ok(target_digest.to_string()))
}

fn failures_with_digest(
    root: &Path,
    value: &Value,
    current: Result<String, String>,
) -> Vec<String> {
    let steps = value
        .get("journey")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    if steps < 10 {
        return vec!["plugin_product_journey_incomplete".to_string()];
    }
    let mut out = evidence_count_failures(value);
    out.extend(authority_failures(value, current));
    if str_field(value, "claim_ceiling") != "package_static_fixture_only" {
        out.push("plugin_product_journey_claim_ceiling_missing".to_string());
    }
    if str_field(value, "generated_at").contains("2026-06-24T00:00:00Z") {
        out.push("plugin_product_journey_placeholder_timestamp".to_string());
    }
    out.extend(evidence_ref_failures(root, value));
    out
}

fn evidence_count_failures(value: &Value) -> Vec<String> {
    let evidence = value
        .get("evidence")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if evidence.len() < 3
        || evidence
            .iter()
            .filter(|item| str_field(item, "path") != FIT_REPO_RECEIPT)
            .count()
            < 2
    {
        vec!["plugin_product_journey_evidence_incomplete".to_string()]
    } else {
        Vec::new()
    }
}

fn authority_failures(value: &Value, current: Result<String, String>) -> Vec<String> {
    let mut out = Vec::new();
    if str_field(value, "status") != "pass" {
        out.push("plugin_product_journey_status_not_pass".to_string());
    }
    if value
        .pointer("/target_revision/kind")
        .and_then(Value::as_str)
        != Some("package_digest")
    {
        out.push("plugin_product_journey_target_revision_not_package_digest".to_string());
    }
    match current {
        Ok(current)
            if value
                .pointer("/target_revision/value")
                .and_then(Value::as_str)
                == Some(current.as_str()) => {}
        Ok(_) => out.push("plugin_product_journey_target_digest_mismatch".to_string()),
        Err(err) => out.push(format!(
            "plugin_product_journey_target_digest_unavailable:{err}"
        )),
    }
    out
}

fn evidence_ref_failures(root: &Path, value: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for item in value
        .get("evidence")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Err(err) =
            crate::package::artifact::refs::validate_object(root, item, "plugin journey evidence")
        {
            out.push(format!("plugin_product_journey_evidence_invalid:{err}"));
        }
    }
    if let Some(item) = value.get("error_path_evidence") {
        if let Err(err) =
            crate::package::artifact::refs::validate_object(root, item, "plugin journey error path")
        {
            out.push(format!("plugin_product_journey_evidence_invalid:{err}"));
        }
    } else {
        out.push("plugin_product_journey_error_path_missing".to_string());
    }
    out
}

fn str_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}
