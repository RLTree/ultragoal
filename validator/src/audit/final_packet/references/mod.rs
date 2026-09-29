use crate::{json_boundary, schema_catalog};
use serde_json::Value;
use std::path::Path;

mod coverage;
mod package;
mod source_audit;

pub(super) fn failures(
    root: &Path,
    store: &schema_catalog::SchemaStore,
    receipt: &Value,
) -> Vec<String> {
    let expected = crate::package::inventory::package_digest(root).unwrap_or_default();
    let mut out = Vec::new();
    check_performance_ref(root, receipt, &expected, &mut out);
    check_registry_ref(root, store, receipt, &mut out);
    source_audit::check_ref(root, receipt, &expected, &mut out);
    check_coverage_ref(root, receipt, &expected, &mut out);
    package::failures(root, receipt, &expected, &mut out);
    out
}

pub(super) fn claim_guard_failures(
    root: &Path,
    store: &schema_catalog::SchemaStore,
    receipt: &Value,
) -> Vec<String> {
    let expected = crate::package::inventory::package_digest(root).unwrap_or_default();
    let mut out = Vec::new();
    check_performance_ref(root, receipt, &expected, &mut out);
    check_registry_guard_ref(root, store, receipt, &mut out);
    source_audit::check_guard_ref(root, receipt, &expected, &mut out);
    check_coverage_ref(root, receipt, &expected, &mut out);
    package::failures(root, receipt, &expected, &mut out);
    out
}

fn check_performance_ref(root: &Path, receipt: &Value, expected: &str, out: &mut Vec<String>) {
    let Some(value) = load_pass_ref(root, receipt, "/cli_performance", "cli_performance", out)
    else {
        return;
    };
    for failure in
        crate::audit::cli::performance::receipt::same_candidate_pass_failures(&value, expected)
    {
        out.push(format!("final_packet_proof_cli_performance_ref:{failure}"));
    }
}

fn check_registry_ref(
    root: &Path,
    store: &schema_catalog::SchemaStore,
    receipt: &Value,
    out: &mut Vec<String>,
) {
    let Some(value) = load_pass_ref(root, receipt, "/registry_exposure", "registry", out) else {
        return;
    };
    for failure in crate::audit::plugin::registry::value_failures(root, store, &value) {
        out.push(format!("final_packet_proof_registry_ref:{failure}"));
    }
}

fn check_registry_guard_ref(
    root: &Path,
    store: &schema_catalog::SchemaStore,
    receipt: &Value,
    out: &mut Vec<String>,
) {
    let Some(value) = load_ref(
        root,
        receipt,
        "/registry_exposure",
        "registry",
        RefStatusPolicy::PassOrFail,
        out,
    ) else {
        return;
    };
    for failure in crate::audit::plugin::registry::value_claim_guard_failures(root, store, &value) {
        out.push(format!("final_packet_proof_registry_ref:{failure}"));
    }
}

fn check_coverage_ref(root: &Path, receipt: &Value, expected: &str, out: &mut Vec<String>) {
    if let Some(value) = load_pass_ref(root, receipt, "/coverage", "coverage", out) {
        out.extend(coverage::failures(&value, expected));
    }
}

fn load_pass_ref(
    root: &Path,
    receipt: &Value,
    ptr: &str,
    label: &str,
    out: &mut Vec<String>,
) -> Option<Value> {
    load_ref(root, receipt, ptr, label, RefStatusPolicy::MustPass, out)
}

#[derive(Clone, Copy)]
pub(super) enum RefStatusPolicy {
    MustPass,
    PassOrFail,
}

pub(super) fn load_ref(
    root: &Path,
    receipt: &Value,
    ptr: &str,
    label: &str,
    status_policy: RefStatusPolicy,
    out: &mut Vec<String>,
) -> Option<Value> {
    let Some(item) = receipt.pointer(ptr) else {
        out.push(format!("final_packet_proof_ref_missing:{label}"));
        return None;
    };
    if label != "source_audit" && item.get("self_rewriting_authority").is_some() {
        out.push(format!(
            "final_packet_proof_ref_unexpected_self_rewrite_authority:{label}"
        ));
        return None;
    }
    let rel = item.get("path").and_then(Value::as_str).unwrap_or("");
    if crate::package::inventory::package_path_error(root, rel).is_some() {
        out.push(format!("final_packet_proof_ref_path_invalid:{label}:{rel}"));
        return None;
    }
    let expected_digest = item.get("digest").and_then(Value::as_str).unwrap_or("");
    match crate::digest::file(&root.join(rel)) {
        Ok(actual) if actual == expected_digest => {}
        _ => {
            out.push(format!(
                "final_packet_proof_ref_digest_mismatch:{label}:{rel}"
            ));
            return None;
        }
    }
    let value = match json_boundary::read_json(&root.join(rel)) {
        Ok(value) => value,
        Err(err) => {
            out.push(format!("final_packet_proof_ref_malformed:{label}:{err}"));
            return None;
        }
    };
    embedded_status_failures(item, &value, label, status_policy, out);
    Some(value)
}

fn embedded_status_failures(
    item: &Value,
    value: &Value,
    label: &str,
    status_policy: RefStatusPolicy,
    out: &mut Vec<String>,
) {
    let embedded_status = item.get("status").and_then(Value::as_str);
    match status_policy {
        RefStatusPolicy::MustPass if embedded_status != Some("pass") => {
            out.push(format!(
                "final_packet_proof_ref_embedded_status_not_pass:{label}"
            ));
        }
        RefStatusPolicy::PassOrFail if !matches!(embedded_status, Some("pass" | "fail")) => {
            out.push(format!(
                "final_packet_proof_ref_embedded_status_not_pass_or_fail:{label}"
            ));
        }
        _ => {}
    }
    if let Some(actual) = value.get("status").and_then(Value::as_str)
        && embedded_status != Some(actual)
    {
        out.push(format!(
            "final_packet_proof_ref_status_disagreement:{label}"
        ));
    }
}
