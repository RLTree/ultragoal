use super::operation::SurfaceOperation;
use serde_json::Value;
use std::path::Path;

const SCHEMA: &str = "harness-ultragoal.package-surface-audit-receipt.v1";
const SCHEMA_FILE: &str = "package-surface-audit-receipt.schema.json";

pub(super) fn receipt_failures(
    root: &Path,
    value: &Value,
    expected_candidate: &str,
    operation: SurfaceOperation,
) -> Vec<String> {
    let store = crate::schema_catalog::load(root);
    let mut out = crate::schema_catalog::schema_errors(&store, SCHEMA_FILE, value)
        .into_iter()
        .map(|failure| format!("package_surface_audit_schema:{failure}"))
        .collect::<Vec<_>>();
    out.extend(same_candidate_pass_or_fail_closed_failures(
        value,
        expected_candidate,
        operation,
    ));
    out
}

fn same_candidate_pass_or_fail_closed_failures(
    value: &Value,
    expected_candidate: &str,
    operation: SurfaceOperation,
) -> Vec<String> {
    if value.get("status").and_then(Value::as_str) == Some("pass") {
        return same_candidate_pass_failures(value, expected_candidate, operation);
    }
    let mut out = base_failures(value, operation, expected_candidate);
    if value.get("status").and_then(Value::as_str) != Some("fail") {
        out.push("package_surface_audit_status_not_pass_or_fail".to_string());
    }
    if value.get("claim_ceiling").and_then(Value::as_str) != Some("withheld_or_blocked") {
        out.push("package_surface_audit_fail_closed_claim_ceiling_not_blocking".to_string());
    }
    if value.get("same_candidate").and_then(Value::as_bool) != Some(false) {
        out.push("package_surface_audit_fail_closed_same_candidate_not_false".to_string());
    }
    if value
        .get("failures")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty)
    {
        out.push("package_surface_audit_fail_closed_missing_failures".to_string());
    }
    for claim in [
        "install_cache_parity",
        "package_readiness",
        "review_readiness",
        "release_readiness",
        "completion",
        "update_goal_eligibility",
    ] {
        if !claim_array_contains(value, "blocked_claim_classes", claim) {
            out.push(format!(
                "package_surface_audit_fail_closed_missing_blocked_claim:{claim}"
            ));
        }
    }
    out
}

fn same_candidate_pass_failures(
    value: &Value,
    expected_candidate: &str,
    operation: SurfaceOperation,
) -> Vec<String> {
    let mut out = surface_failures(value, operation, expected_candidate);
    out.push("package_surface_audit_independent_observation_unavailable".to_string());
    if value.get("status").and_then(Value::as_str) != Some("pass") {
        out.push("package_surface_audit_status_not_pass".to_string());
    }
    if value.get("claim_ceiling").and_then(Value::as_str) != Some("surface_package_digest_aligned")
    {
        out.push("package_surface_audit_claim_ceiling_not_surface_aligned".to_string());
    }
    if value.get("same_candidate").and_then(Value::as_bool) != Some(true) {
        out.push("package_surface_audit_not_same_candidate".to_string());
    }
    out
}

fn surface_failures(
    value: &Value,
    operation: SurfaceOperation,
    expected_candidate: &str,
) -> Vec<String> {
    let mut out = base_failures(value, operation, expected_candidate);
    if value
        .pointer("/target/package_digest")
        .and_then(Value::as_str)
        != Some(expected_candidate)
    {
        out.push("package_surface_audit_target_digest_mismatch".to_string());
    }
    out
}

fn base_failures(
    value: &Value,
    operation: SurfaceOperation,
    expected_candidate: &str,
) -> Vec<String> {
    let mut out = Vec::new();
    if value.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
        out.push("package_surface_audit_wrong_schema".to_string());
    }
    if value.get("operation").and_then(Value::as_str) != Some(operation.id()) {
        out.push("package_surface_audit_wrong_operation".to_string());
    }
    if value.get("candidate_digest").and_then(Value::as_str) != Some(expected_candidate) {
        out.push("package_surface_audit_candidate_digest_mismatch".to_string());
    }
    if value
        .pointer("/source/package_digest")
        .and_then(Value::as_str)
        != Some(expected_candidate)
    {
        out.push("package_surface_audit_source_digest_mismatch".to_string());
    }
    if value
        .pointer("/target/expected_package_digest")
        .and_then(Value::as_str)
        != Some(expected_candidate)
    {
        out.push("package_surface_audit_target_expected_digest_mismatch".to_string());
    }
    if value.pointer("/target/surface").and_then(Value::as_str) != Some(operation.surface_id()) {
        out.push("package_surface_audit_target_surface_mismatch".to_string());
    }
    if value.pointer("/target/local_path").is_some() {
        out.push("package_surface_audit_private_local_path_present".to_string());
    }
    out
}

fn claim_array_contains(value: &Value, key: &str, expected: &str) -> bool {
    value
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(expected)))
}

#[cfg(test)]
mod tests {
    use super::SurfaceOperation;
    use serde_json::json;

    #[test]
    fn receipt_failures_prefix_schema_errors() {
        let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
        let candidate = crate::self_tests::boundaries::workspace_fixtures::sha('a');
        let failures = super::receipt_failures(
            &root,
            &json!({"schema":"wrong"}),
            &candidate,
            SurfaceOperation::InstallAudit,
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.starts_with("package_surface_audit_schema:")),
            "{failures:?}"
        );
    }

    #[test]
    fn copied_digests_cannot_prove_a_package_surface() {
        let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
        let candidate = crate::self_tests::boundaries::workspace_fixtures::sha('a');
        let receipt = json!({
            "schema": "harness-ultragoal.package-surface-audit-receipt.v1",
            "operation": "install_audit",
            "candidate_digest": candidate,
            "status": "pass",
            "claim_ceiling": "surface_package_digest_aligned",
            "same_candidate": true,
            "source": {"package_digest": candidate},
            "target": {
                "expected_package_digest": candidate,
                "package_digest": candidate,
                "surface": "installed_plugin"
            }
        });
        let failures =
            super::receipt_failures(&root, &receipt, &candidate, SurfaceOperation::InstallAudit);
        assert!(
            failures
                .contains(&"package_surface_audit_independent_observation_unavailable".to_string()),
            "{failures:?}"
        );
    }

    #[test]
    fn unavailable_surface_can_still_preserve_a_fail_closed_ceiling() {
        let candidate = crate::self_tests::boundaries::workspace_fixtures::sha('a');
        let receipt = json!({
            "schema": "harness-ultragoal.package-surface-audit-receipt.v1",
            "operation": "install_audit",
            "candidate_digest": candidate,
            "status": "fail",
            "claim_ceiling": "withheld_or_blocked",
            "same_candidate": false,
            "failures": ["surface was not independently observed"],
            "blocked_claim_classes": [
                "install_cache_parity",
                "package_readiness",
                "review_readiness",
                "release_readiness",
                "completion",
                "update_goal_eligibility"
            ],
            "source": {"package_digest": candidate},
            "target": {
                "expected_package_digest": candidate,
                "surface": "installed_plugin"
            }
        });
        let failures = super::same_candidate_pass_or_fail_closed_failures(
            &receipt,
            &candidate,
            SurfaceOperation::InstallAudit,
        );
        assert!(failures.is_empty(), "{failures:?}");
    }
}
