use serde_json::Value;

mod evidence;

const RECEIPT_SCHEMA: &str = "harness-ultragoal.cli-control-plane-receipt.v1";

pub(crate) fn surface_value_failures(value: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if value.get("schema").and_then(Value::as_str) != Some(RECEIPT_SCHEMA) {
        out.push("cli_control_plane_receipt_wrong_schema".to_string());
    }
    if value.pointer("/issuer/tool").and_then(Value::as_str) != Some("ultragoal") {
        out.push("cli_control_plane_receipt_wrong_issuer".to_string());
    }
    if value.get("operation").and_then(Value::as_str).is_none() {
        out.push("cli_control_plane_receipt_missing_operation".to_string());
    }
    if value
        .get("candidate_digest")
        .and_then(Value::as_str)
        .is_none()
    {
        out.push("cli_control_plane_receipt_missing_candidate_digest".to_string());
    }
    if value
        .get("blocked_claim_classes")
        .and_then(Value::as_array)
        .is_none()
    {
        out.push("cli_control_plane_receipt_missing_blocked_claims".to_string());
    }
    let failure_law = value.pointer("/failure/law_id").and_then(Value::as_str);
    if !matches!(
        failure_law,
        Some("cli-control-plane-authority" | "cli-self-law-compliance")
    ) && value.get("failure") != Some(&Value::Null)
    {
        out.push("cli_control_plane_receipt_missing_self_law_failure".to_string());
    }
    out.extend(evidence::receipt_surface_failures(value));
    out
}

pub(crate) fn same_candidate_pass_failures(
    value: &Value,
    expected_candidate: &str,
    expected_operation: &str,
) -> Vec<String> {
    let mut out = surface_value_failures(value);
    out.push("cli_control_plane_receipt_independent_authority_unavailable".to_string());
    let candidate = value
        .get("candidate_digest")
        .and_then(Value::as_str)
        .unwrap_or("");
    if candidate != expected_candidate {
        out.push(format!(
            "cli_control_plane_receipt_candidate_digest_mismatch:{candidate}!={expected_candidate}"
        ));
    }
    if value.get("operation").and_then(Value::as_str) != Some(expected_operation) {
        out.push(format!(
            "cli_control_plane_receipt_wrong_operation:{expected_operation}"
        ));
    }
    if value.get("status").and_then(Value::as_str) != Some("pass") {
        out.push("cli_control_plane_receipt_not_pass".to_string());
    }
    if value
        .pointer("/issuer/self_law_state")
        .and_then(Value::as_str)
        != Some("self_hosted")
    {
        out.push("cli_control_plane_receipt_not_self_hosted".to_string());
    }
    if value.get("claim_ceiling").and_then(Value::as_str)
        != Some("supports_update_goal_eligibility")
    {
        out.push("cli_control_plane_receipt_claim_ceiling_not_update_goal".to_string());
    }
    if !value
        .get("blocked_claim_classes")
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty)
    {
        out.push("cli_control_plane_receipt_blocks_claims".to_string());
    }
    if value
        .get("failure")
        .is_some_and(|failure| failure != &Value::Null)
    {
        out.push("cli_control_plane_receipt_pass_has_failure".to_string());
    }
    if candidate != expected_candidate
        || value.get("operation").and_then(Value::as_str) != Some(expected_operation)
    {
        return out;
    }
    out.extend(evidence::same_candidate_pass_failures(
        value,
        expected_candidate,
        expected_operation,
    ));
    out
}

pub(crate) fn same_candidate_fail_closed_failures(
    value: &Value,
    expected_candidate: &str,
    expected_operation: &str,
) -> Vec<String> {
    let mut out = surface_value_failures(value);
    let candidate = value
        .get("candidate_digest")
        .and_then(Value::as_str)
        .unwrap_or("");
    if candidate != expected_candidate {
        out.push(format!(
            "cli_control_plane_receipt_candidate_digest_mismatch:{candidate}!={expected_candidate}"
        ));
    }
    if value.get("operation").and_then(Value::as_str) != Some(expected_operation) {
        out.push(format!(
            "cli_control_plane_receipt_wrong_operation:{expected_operation}"
        ));
    }
    if value.get("status").and_then(Value::as_str) != Some("fail") {
        out.push("cli_control_plane_receipt_not_fail_closed".to_string());
    }
    if value
        .pointer("/issuer/self_law_state")
        .and_then(Value::as_str)
        != Some("transition_only")
    {
        out.push("cli_control_plane_receipt_not_transition_only".to_string());
    }
    if value.get("claim_ceiling").and_then(Value::as_str) != Some("withheld_or_blocked") {
        out.push("cli_control_plane_receipt_claim_ceiling_not_blocking".to_string());
    }
    for claim in [
        "completion",
        "package_readiness",
        "review_readiness",
        "release_readiness",
        "update_goal_eligibility",
    ] {
        if !blocked_claims_contain(value, claim) {
            out.push(format!(
                "cli_control_plane_receipt_missing_blocked_claim:{claim}"
            ));
        }
    }
    match value.get("failure") {
        Some(Value::Object(failure)) => {
            if !failure
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| id.ends_with("_evidence_not_satisfied"))
            {
                out.push("cli_control_plane_receipt_failure_not_evidence_bound".to_string());
            }
        }
        _ => out.push("cli_control_plane_receipt_missing_fail_closed_failure".to_string()),
    }
    if candidate != expected_candidate
        || value.get("operation").and_then(Value::as_str) != Some(expected_operation)
    {
        return out;
    }
    out.extend(evidence::same_candidate_fail_closed_failures(
        value,
        expected_candidate,
        expected_operation,
    ));
    out
}

fn blocked_claims_contain(value: &Value, claim: &str) -> bool {
    value
        .get("blocked_claim_classes")
        .and_then(Value::as_array)
        .is_some_and(|claims| claims.iter().any(|item| item.as_str() == Some(claim)))
}
#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn repository_receipt_cannot_mint_pass_authority() {
        let candidate = crate::self_tests::boundaries::workspace_fixtures::sha('a');
        let receipt = json!({
            "schema": "harness-ultragoal.cli-control-plane-receipt.v1",
            "issuer": {"tool": "ultragoal", "self_law_state": "self_hosted"},
            "operation": "update_goal_eligibility",
            "candidate_digest": candidate,
            "status": "pass",
            "claim_ceiling": "supports_update_goal_eligibility",
            "blocked_claim_classes": [],
            "failure": null,
            "evidence_graph": {
                "evaluation_mode": "production_dereferenced",
                "candidate_digest": candidate,
                "operation": "update_goal_eligibility",
                "operation_failures": [],
                "items": []
            }
        });
        let failures =
            super::same_candidate_pass_failures(&receipt, &candidate, "update_goal_eligibility");
        assert!(
            failures.contains(
                &"cli_control_plane_receipt_independent_authority_unavailable".to_string()
            ),
            "{failures:?}"
        );
    }

    #[test]
    fn evidence_failure_can_still_preserve_a_fail_closed_ceiling() {
        let candidate = crate::self_tests::boundaries::workspace_fixtures::sha('a');
        let receipt = json!({
            "schema": "harness-ultragoal.cli-control-plane-receipt.v1",
            "issuer": {"tool": "ultragoal", "self_law_state": "transition_only"},
            "operation": "registry_probe",
            "candidate_digest": candidate,
            "status": "fail",
            "claim_ceiling": "withheld_or_blocked",
            "blocked_claim_classes": [
                "completion",
                "package_readiness",
                "review_readiness",
                "release_readiness",
                "update_goal_eligibility"
            ],
            "failure": {
                "id": "registry_probe_evidence_not_satisfied",
                "law_id": "cli-control-plane-authority"
            },
            "evidence_graph": {
                "evaluation_mode": "production_dereferenced",
                "candidate_digest": candidate,
                "operation": "registry_probe",
                "operation_failures": [{"id": "registry unavailable"}],
                "items": [{"label": "registry_exposure"}]
            }
        });
        let failures =
            super::same_candidate_fail_closed_failures(&receipt, &candidate, "registry_probe");
        assert!(failures.is_empty(), "{failures:?}");
    }
}
