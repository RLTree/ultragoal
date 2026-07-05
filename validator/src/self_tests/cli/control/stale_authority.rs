use crate::cli::control::plane::RECEIPT_SCHEMA;
use crate::cli::control::plane::receipt::{
    same_candidate_fail_closed_failures, same_candidate_pass_failures,
};
use serde_json::json;

#[test]
fn stale_control_receipt_does_not_project_nested_evidence_failures() {
    let stale = crate::self_tests::boundaries::workspace_fixtures::sha('s');
    let current = crate::self_tests::boundaries::workspace_fixtures::sha('c');
    let value = json!({
        "schema": RECEIPT_SCHEMA,
        "issuer": {"tool": "ultragoal", "self_law_state": "self_hosted"},
        "operation": "update_goal_eligibility",
        "candidate_digest": stale,
        "status": "pass",
        "claim_ceiling": "supports_update_goal_eligibility",
        "blocked_claim_classes": [],
        "failure": null,
        "evidence_graph": graph_with_product_failures(&stale, "update_goal_eligibility")
    });

    let failures = same_candidate_pass_failures(&value, &current, "update_goal_eligibility");
    assert!(
        failures
            .iter()
            .any(|failure| failure
                .starts_with("cli_control_plane_receipt_candidate_digest_mismatch"))
    );
    assert_nested_product_failures_are_hidden(&failures);
}

#[test]
fn stale_fail_closed_control_receipt_stops_at_authority_boundary() {
    let stale = crate::self_tests::boundaries::workspace_fixtures::sha('t');
    let current = crate::self_tests::boundaries::workspace_fixtures::sha('d');
    let value = json!({
        "schema": RECEIPT_SCHEMA,
        "issuer": {"tool": "ultragoal", "self_law_state": "transition_only"},
        "operation": "self_update_goal_eligibility",
        "candidate_digest": stale,
        "status": "fail",
        "claim_ceiling": "withheld_or_blocked",
        "blocked_claim_classes": [
            "completion",
            "package_readiness",
            "review_readiness",
            "release_readiness",
            "final_packet_correctness",
            "update_goal_eligibility"
        ],
        "failure": {
            "id": "self_update_goal_eligibility_evidence_not_satisfied",
            "law_id": "cli-control-plane-authority"
        },
        "evidence_graph": graph_with_product_failures(&stale, "self_update_goal_eligibility")
    });

    let failures =
        same_candidate_fail_closed_failures(&value, &current, "self_update_goal_eligibility");
    assert!(
        failures
            .iter()
            .any(|failure| failure
                .starts_with("cli_control_plane_receipt_candidate_digest_mismatch"))
    );
    assert_nested_product_failures_are_hidden(&failures);
}

#[test]
fn malformed_current_receipt_reports_missing_evidence_graph() {
    let current = crate::self_tests::boundaries::workspace_fixtures::sha('m');
    let value = json!({
        "schema": RECEIPT_SCHEMA,
        "issuer": {"tool": "ultragoal", "self_law_state": "transition_only"},
        "operation": "self_update_goal_eligibility",
        "candidate_digest": current,
        "status": "fail",
        "claim_ceiling": "withheld_or_blocked",
        "blocked_claim_classes": [
            "completion",
            "package_readiness",
            "review_readiness",
            "release_readiness",
            "final_packet_correctness",
            "update_goal_eligibility"
        ],
        "failure": {
            "id": "self_update_goal_eligibility_evidence_not_satisfied",
            "law_id": "cli-control-plane-authority"
        }
    });

    let failures =
        same_candidate_fail_closed_failures(&value, &current, "self_update_goal_eligibility");
    assert!(
        failures
            .iter()
            .any(|failure| failure == "cli_control_plane_receipt_missing_evidence_graph"),
        "{failures:?}"
    );
}

#[test]
fn current_receipt_rejects_mismatched_evidence_graph_binding() {
    let current = crate::self_tests::boundaries::workspace_fixtures::sha('g');
    let mut value = json!({
        "schema": RECEIPT_SCHEMA,
        "issuer": {"tool": "ultragoal", "self_law_state": "self_hosted"},
        "operation": "update_goal_eligibility",
        "candidate_digest": current,
        "status": "pass",
        "claim_ceiling": "supports_update_goal_eligibility",
        "blocked_claim_classes": [],
        "failure": null,
        "evidence_graph": graph_with_product_failures(&current, "update_goal_eligibility")
    });
    value["evidence_graph"]["candidate_digest"] =
        json!(crate::self_tests::boundaries::workspace_fixtures::sha('w'));
    value["evidence_graph"]["operation"] = json!("packet_verify");

    let failures = same_candidate_pass_failures(&value, &current, "update_goal_eligibility");
    for expected in [
        "cli_control_plane_receipt_evidence_graph_wrong_candidate",
        "cli_control_plane_receipt_evidence_graph_wrong_operation",
    ] {
        assert!(
            failures.iter().any(|failure| failure == expected),
            "{expected}: {failures:?}"
        );
    }
}

fn graph_with_product_failures(candidate: &str, operation: &str) -> serde_json::Value {
    json!({
        "candidate_digest": candidate,
        "evaluation_mode": "production_dereferenced",
        "operation": operation,
        "operation_failures": ["fit_repo:stale_product_receipt"],
        "items": [
            failed_item(candidate, "fit_repo"),
            failed_item(candidate, "product_journey")
        ]
    })
}

fn failed_item(candidate: &str, label: &str) -> serde_json::Value {
    json!({
        "candidate_digest": candidate,
        "digest": candidate,
        "exists": true,
        "failures": ["stale_or_wrong_digest_nested_evidence"],
        "label": label,
        "path": format!("validation_artifacts/{label}.json"),
        "same_candidate": true,
        "schema": format!("harness-ultragoal.{label}.v1"),
        "status": "fail"
    })
}

fn assert_nested_product_failures_are_hidden(failures: &[String]) {
    for forbidden in [
        "cli_control_plane_receipt_evidence_graph_has_operation_failures",
        "cli_control_plane_receipt_evidence_graph_item_has_failures:fit_repo",
        "cli_control_plane_receipt_evidence_graph_item_has_failures:product_journey",
        "cli_control_plane_receipt_evidence_graph_item_not_pass:fit_repo",
        "cli_control_plane_receipt_evidence_graph_item_not_pass:product_journey",
    ] {
        assert!(
            failures.iter().all(|failure| failure != forbidden),
            "{forbidden}: {failures:?}"
        );
    }
}
