use serde_json::Value;
use std::path::Path;

pub(super) fn typed_status(
    label: &str,
    value: &Value,
    failures: &[String],
) -> Option<&'static str> {
    if value.get("status").and_then(Value::as_str) == Some("pass") {
        return Some("pass");
    }
    if value.get("status").and_then(Value::as_str) == Some("fail") {
        return Some("fail");
    }
    if inferred_status_label(label) && failures.is_empty() {
        return Some("pass");
    }
    None
}

pub(super) fn label_failures(
    root: &Path,
    label: &str,
    value: &Value,
    expected: &str,
) -> Vec<String> {
    match label {
        "source_audit" => source_audit_failures(value, expected),
        "red_fixture_report" => target_status_failures(value, expected, "red_fixture_report"),
        "coverage" => retired_evidence_failures("coverage"),
        "cli_performance" => {
            crate::cli::performance::receipt::same_candidate_pass_failures(value, expected)
        }
        "final_packet" => {
            let store = crate::schema_catalog::load(root);
            crate::audit::final_packet::value_failures(root, &store, value)
        }
        "registry_exposure" => {
            let store = crate::schema_catalog::load(root);
            crate::audit::plugin::registry::value_failures(root, &store, value)
        }
        "fit_repo" => crate::audit::fit_repo_receipt::failures(root, value),
        "product_fitness" => {
            crate::audit::product::fitness::canonical_package_receipt_value_failures(root, value)
        }
        "product_journey" => {
            crate::audit::plugin::product::cohesion::journey_value_failures(root, value)
        }
        "standards_gardener" => standards_gardener_failures(root, value),
        "install_audit" => package_surface_failures(
            root,
            value,
            expected,
            crate::cli::control::plane::operation::ControlOperation::InstallAudit,
        ),
        "cache_audit" => package_surface_failures(
            root,
            value,
            expected,
            crate::cli::control::plane::operation::ControlOperation::CacheAudit,
        ),
        label if label.starts_with("rust_") => rust_failures(label, value, expected),
        label if label.starts_with("gc_") => gc_failures(value, expected),
        "transactional_finalization" => retired_evidence_failures("transactional_finalization"),
        _ => vec![format!("unknown_evidence_label:{label}")],
    }
}

fn inferred_status_label(label: &str) -> bool {
    matches!(
        label,
        "coverage" | "fit_repo" | "product_fitness" | "standards_gardener"
    )
}

fn source_audit_failures(value: &Value, expected: &str) -> Vec<String> {
    let mut out = target_status_failures(value, expected, "source_audit");
    if value.get("status").and_then(Value::as_str) != Some("pass") {
        out.push("source_audit_status_not_pass".to_string());
    }
    if value.get("claim_ceiling").and_then(Value::as_str)
        != Some("source_audit_pass_source_local_only")
    {
        out.push("source_audit_claim_ceiling_not_source_local".to_string());
    }
    for supported in ["source_local_audit_checks", "red_fixture_report"] {
        if !array_contains(value, "supported_claim_classes", supported) {
            out.push(format!("source_audit_supported_claim_missing:{supported}"));
        }
    }
    for blocked in [
        "completion",
        "package_readiness",
        "review_readiness",
        "release_readiness",
        "final_packet_correctness",
        "update_goal_eligibility",
        "app_registry_or_reviewer_exposure",
    ] {
        if !array_contains(value, "blocked_claim_classes", blocked) {
            out.push(format!("source_audit_missing_blocked_claim:{blocked}"));
        }
    }
    out
}

fn target_status_failures(value: &Value, expected: &str, label: &str) -> Vec<String> {
    let mut out = Vec::new();
    if value
        .pointer("/target_revision/value")
        .and_then(Value::as_str)
        != Some(expected)
    {
        out.push(format!("{label}_target_digest_mismatch"));
    }
    if value.get("status").and_then(Value::as_str) != Some("pass") {
        out.push(format!("{label}_status_not_pass"));
    }
    out
}

fn retired_evidence_failures(label: &str) -> Vec<String> {
    vec![format!("retired_evidence_label:{label}")]
}

fn array_contains(value: &Value, key: &str, needle: &str) -> bool {
    value
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(needle)))
}

fn standards_gardener_failures(root: &Path, value: &Value) -> Vec<String> {
    let store = crate::schema_catalog::load(root);
    let mut out = crate::audit::standards_gardening::receipt_failures(&store, value);
    out.extend(crate::audit::standards_gardening::receipt_root_failures(
        root, value,
    ));
    out
}

fn package_surface_failures(
    root: &Path,
    value: &Value,
    expected: &str,
    operation: crate::cli::control::plane::operation::ControlOperation,
) -> Vec<String> {
    let store = crate::schema_catalog::load(root);
    let mut out = crate::schema_catalog::schema_errors(
        &store,
        crate::cli::control::plane::surface::SCHEMA_FILE,
        value,
    )
    .into_iter()
    .map(|failure| format!("package_surface_audit_schema:{failure}"))
    .collect::<Vec<_>>();
    out.extend(
        crate::cli::control::plane::surface::same_candidate_pass_or_fail_closed_failures(
            value, expected, operation,
        ),
    );
    out
}

fn rust_failures(label: &str, value: &Value, expected: &str) -> Vec<String> {
    let expected_law = match label {
        "rust_dependency" | "rust_workspace_topology" => "rust-developer-experience-authority",
        "rust_toolchain" => "rust-toolchain-substrate-authority",
        "rust_clean_proof" => "rust-cache-no-cache-honesty",
        "rust_memory" => "rust-memory-resource-discipline",
        _ => "rust-command-loop-authority",
    };
    let mut out = crate::cli::rust::receipt::surface_value_failures(value, expected_law);
    if value.pointer("/digests/candidate").and_then(Value::as_str) != Some(expected) {
        out.push("rust_receipt_candidate_digest_mismatch".to_string());
    }
    if value.get("status").and_then(Value::as_str) != Some("pass") {
        out.push("rust_receipt_status_not_pass".to_string());
    }
    out
}

fn gc_failures(value: &Value, expected: &str) -> Vec<String> {
    let mut out = crate::cli::garbage::collection::receipt::surface_value_failures(value);
    if value.pointer("/digests/candidate").and_then(Value::as_str) != Some(expected) {
        out.push("gc_receipt_candidate_digest_mismatch".to_string());
    }
    if value.get("status").and_then(Value::as_str) != Some("pass") {
        out.push("gc_receipt_status_not_pass".to_string());
    }
    out
}

#[cfg(test)]
mod label_failures;
#[cfg(test)]
mod typed_and_surfaces;
