use crate::cli::control::plane::operation::ControlOperation;
use crate::schema_catalog;
use serde_json::Value;
use std::path::Path;

const RECEIPT: &str = "validation_artifacts/cli/transactional-finalization-receipt.json";
const SCHEMA: &str = "harness-ultragoal.cli-transactional-finalization-receipt.v1";
const SCHEMA_FILE: &str = "cli-transactional-finalization-receipt.schema.json";
const FINAL_PACKET: &str = "validation_artifacts/review/final-packet-proof.json";
const REGISTRY_EXPOSURE: &str =
    "validation_artifacts/ultragoal-audit/active-registry-exposure-current.json";
const CLI_PERFORMANCE: &str = "validation_artifacts/cli/performance-receipt.json";
const COVERAGE: &str = "validation_artifacts/coverage/coverage-receipt.json";

pub(super) fn failures(root: &Path, operation: ControlOperation) -> Vec<String> {
    if !requires_transaction(operation) {
        return Vec::new();
    }
    let expected = match crate::package::inventory::package_digest(root) {
        Ok(digest) => digest,
        Err(err) => {
            return vec![format!(
                "cli_control_plane_transaction_digest_unavailable:{err}"
            )];
        }
    };
    let receipt = match crate::json_boundary::read_json(&root.join(RECEIPT)) {
        Ok(value) => value,
        Err(err) => {
            return vec![format!(
                "cli_control_plane_transactional_finalization_missing:{RECEIPT}:{err}"
            )];
        }
    };
    let store = crate::schema_catalog::load(root);
    let mut out = store
        .errors
        .iter()
        .map(|failure| format!("cli_control_plane_transaction_schema_catalog:{failure}"))
        .collect::<Vec<_>>();
    out.extend(
        crate::schema_catalog::schema_errors(&store, SCHEMA_FILE, &receipt)
            .into_iter()
            .map(|failure| format!("cli_control_plane_transaction_schema:{failure}")),
    );
    out.extend(receipt_failures(root, &store, &receipt, &expected));
    out
}

fn requires_transaction(operation: ControlOperation) -> bool {
    matches!(
        operation,
        ControlOperation::UpdateGoalEligibility | ControlOperation::SelfUpdateGoalEligibility
    )
}

pub(crate) fn receipt_failures(
    root: &Path,
    store: &schema_catalog::SchemaStore,
    receipt: &Value,
    expected: &str,
) -> Vec<String> {
    let mut out = Vec::new();
    if receipt.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
        out.push("cli_control_plane_transaction_wrong_schema".to_string());
    }
    if receipt.get("status").and_then(Value::as_str) != Some("pass") {
        out.push("cli_control_plane_transaction_status_not_pass".to_string());
    }
    if receipt.get("candidate_digest").and_then(Value::as_str) != Some(expected) {
        out.push("cli_control_plane_transaction_candidate_digest_mismatch".to_string());
    }
    if receipt.get("claim_ceiling").and_then(Value::as_str)
        != Some("supports_update_goal_eligibility")
    {
        out.push("cli_control_plane_transaction_claim_ceiling_not_update_goal".to_string());
    }
    if receipt.get("transaction_mode").and_then(Value::as_str)
        != Some("same_candidate_atomic_finalization")
    {
        out.push("cli_control_plane_transaction_mode_not_atomic".to_string());
    }
    if !receipt
        .get("blocked_claim_classes")
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty)
    {
        out.push("cli_control_plane_transaction_blocks_claims".to_string());
    }
    for (ptr, rel) in [
        ("/final_packet", FINAL_PACKET),
        ("/registry_exposure", REGISTRY_EXPOSURE),
        ("/cli_performance", CLI_PERFORMANCE),
        ("/coverage", COVERAGE),
    ] {
        ref_failures(root, &store, receipt, ptr, rel, expected, &mut out);
    }
    out
}

pub(crate) fn same_candidate_failures(
    root: &Path,
    store: &schema_catalog::SchemaStore,
    receipt: &Value,
    expected: &str,
) -> Vec<String> {
    let mut out = schema_catalog::schema_errors(store, SCHEMA_FILE, receipt)
        .into_iter()
        .map(|failure| format!("cli_control_plane_transaction_schema:{failure}"))
        .collect::<Vec<_>>();
    out.extend(receipt_failures(root, store, receipt, expected));
    out
}

fn ref_failures(
    root: &Path,
    store: &schema_catalog::SchemaStore,
    receipt: &Value,
    ptr: &str,
    required_rel: &str,
    expected: &str,
    out: &mut Vec<String>,
) {
    let label = ptr.trim_start_matches('/');
    let Some(item) = receipt.pointer(ptr) else {
        out.push(format!("cli_control_plane_transaction_ref_missing:{label}"));
        return;
    };
    let rel = item.get("path").and_then(Value::as_str).unwrap_or("");
    if rel != required_rel {
        out.push(format!(
            "cli_control_plane_transaction_ref_wrong_surface:{label}:{rel}"
        ));
        return;
    }
    if crate::package::inventory::package_path_error(root, rel).is_some() {
        out.push(format!(
            "cli_control_plane_transaction_ref_path_invalid:{label}:{rel}"
        ));
        return;
    }
    let expected_digest = item.get("digest").and_then(Value::as_str).unwrap_or("");
    match crate::digest::file(&root.join(rel)) {
        Ok(actual) if actual == expected_digest => {}
        _ => out.push(format!(
            "cli_control_plane_transaction_ref_digest_mismatch:{}:{rel}",
            ptr.trim_start_matches('/')
        )),
    }
    if item.get("status").and_then(Value::as_str) != Some("pass") {
        out.push(format!(
            "cli_control_plane_transaction_ref_status_not_pass:{label}"
        ));
    }
    let value = match crate::json_boundary::read_json(&root.join(rel)) {
        Ok(value) => value,
        Err(err) => {
            out.push(format!(
                "cli_control_plane_transaction_ref_malformed:{label}:{err}"
            ));
            return;
        }
    };
    if let Some(actual) = value.get("status").and_then(Value::as_str) {
        if item.get("status").and_then(Value::as_str) != Some(actual) {
            out.push(format!(
                "cli_control_plane_transaction_ref_status_disagreement:{label}"
            ));
        }
    }
    out.extend(ref_value_failures(root, store, label, &value, expected));
}

fn ref_value_failures(
    root: &Path,
    store: &schema_catalog::SchemaStore,
    label: &str,
    value: &Value,
    expected: &str,
) -> Vec<String> {
    match label {
        "final_packet" => crate::audit::final_packet::package_failures(root, store)
            .into_iter()
            .map(|failure| format!("cli_control_plane_transaction_final_packet:{failure}"))
            .collect(),
        "registry_exposure" => crate::audit::plugin::registry::value_failures(root, store, value)
            .into_iter()
            .map(|failure| format!("cli_control_plane_transaction_registry:{failure}"))
            .collect(),
        "cli_performance" => {
            crate::cli::performance::receipt::same_candidate_pass_failures(value, expected)
                .into_iter()
                .map(|failure| format!("cli_control_plane_transaction_performance:{failure}"))
                .collect()
        }
        "coverage" => coverage_failures(root, store, value, expected),
        _ => vec![format!("cli_control_plane_transaction_ref_unknown:{label}")],
    }
}

#[cfg(test)]
pub(crate) fn unknown_ref_value_failures_for_test(root: &Path) -> Vec<String> {
    let store = crate::schema_catalog::load(root);
    ref_value_failures(root, &store, "unknown", &Value::Null, crate::digest::ZERO)
}

fn coverage_failures(
    root: &Path,
    store: &schema_catalog::SchemaStore,
    value: &Value,
    expected: &str,
) -> Vec<String> {
    let mut out = schema_catalog::schema_errors(store, "coverage-receipt.schema.json", value)
        .into_iter()
        .map(|failure| format!("cli_control_plane_transaction_coverage_schema:{failure}"))
        .collect::<Vec<_>>();
    out.extend(crate::cli::coverage::exact_receipt::claim_failures(
        root,
        value,
        expected,
        &crate::cli::coverage::exact_receipt::TRANSACTION_CODES,
    ));
    out
}
