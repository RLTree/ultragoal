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
        "coverage" => coverage_failures(value, expected),
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
            crate::cli::control::plane::types::ControlOperation::InstallAudit,
        ),
        "cache_audit" => package_surface_failures(
            root,
            value,
            expected,
            crate::cli::control::plane::types::ControlOperation::CacheAudit,
        ),
        label if label.starts_with("rust_") => rust_failures(label, value, expected),
        label if label.starts_with("gc_") => gc_failures(value, expected),
        "transactional_finalization" => Vec::new(),
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

fn coverage_failures(value: &Value, expected: &str) -> Vec<String> {
    let mut out = Vec::new();
    if value
        .pointer("/target_revision/value")
        .and_then(Value::as_str)
        != Some(expected)
    {
        out.push("coverage_target_digest_mismatch".to_string());
    }
    if value.pointer("/coverage/percent").and_then(Value::as_f64) != Some(100.0)
        || !value
            .get("uncovered_records")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
    {
        out.push("coverage_not_exact_100".to_string());
    }
    if value.get("claim_ceiling").and_then(Value::as_str) != Some("supports_complete_claim") {
        out.push("coverage_claim_ceiling_not_complete".to_string());
    }
    out
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
    operation: crate::cli::control::plane::types::ControlOperation,
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
