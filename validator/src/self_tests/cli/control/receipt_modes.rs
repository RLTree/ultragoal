use crate::cli::control::plane::receipt::same_candidate_fail_closed_failures;
use crate::cli::control::plane::receipt::same_candidate_pass_failures;
use crate::cli::control::plane::receipt::surface_value_failures;
use crate::cli::control::plane::types::ControlOperation;
use crate::cli::control::plane::{RECEIPT_SCHEMA, receipt_from_evidence};
use serde_json::json;

#[test]
fn constructor_empty_evidence_branch_is_not_production_green_proof() {
    let candidate = crate::self_tests::boundaries::workspace_fixtures::sha('c');
    let value = receipt_from_evidence(
        candidate.clone(),
        ControlOperation::UpdateGoalEligibility,
        Vec::new(),
    );

    assert_eq!(value["status"], "fail");
    assert_eq!(value["candidate_digest"], candidate);
    assert_eq!(value["issuer"]["self_law_state"], "transition_only");
    assert_eq!(value["claim_ceiling"], "withheld_or_blocked");
    assert_eq!(
        value["failure"]["id"],
        "update_goal_eligibility_evidence_not_satisfied"
    );
    assert!(
        value["failure"]["observed_value"]
            .as_str()
            .expect("observed")
            .contains("cli_control_plane_transactional_green_path_not_proven")
    );
    assert_eq!(
        value["blocked_claim_classes"],
        json!([
            "completion",
            "package_readiness",
            "review_readiness",
            "release_readiness",
            "final_packet_correctness",
            "update_goal_eligibility"
        ])
    );
}

#[test]
fn fail_closed_receipts_are_blockers_not_completion_green_paths() {
    let candidate = crate::self_tests::boundaries::workspace_fixtures::sha('b');
    let fail_closed = json!({
        "schema": RECEIPT_SCHEMA,
        "issuer": {"tool": "ultragoal", "self_law_state": "transition_only"},
        "operation": "update_goal_eligibility",
        "candidate_digest": candidate,
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
            "id": "update_goal_eligibility_evidence_not_satisfied",
            "law_id": "cli-control-plane-authority"
        },
        "evidence_graph": super::graph::control(&candidate, "update_goal_eligibility", true)
    });
    assert!(
        same_candidate_fail_closed_failures(&fail_closed, &candidate, "update_goal_eligibility")
            .is_empty()
    );

    let pass_shaped = json!({
        "schema": RECEIPT_SCHEMA,
        "issuer": {"tool": "ultragoal", "self_law_state": "self_hosted"},
        "operation": "update_goal_eligibility",
        "candidate_digest": candidate,
        "status": "pass",
        "claim_ceiling": "supports_update_goal_eligibility",
        "blocked_claim_classes": [],
        "failure": null,
        "evidence_graph": super::graph::control(&candidate, "update_goal_eligibility", false)
    });
    let failures =
        same_candidate_fail_closed_failures(&pass_shaped, &candidate, "update_goal_eligibility");
    for expected in [
        "cli_control_plane_receipt_not_fail_closed",
        "cli_control_plane_receipt_not_transition_only",
    ] {
        assert!(
            failures.iter().any(|failure| failure == expected),
            "{expected}: {failures:?}"
        );
    }
}

#[test]
fn receipt_mode_validators_reject_wrong_operation_and_weak_failure_ids() {
    let candidate = crate::self_tests::boundaries::workspace_fixtures::sha('d');
    let wrong_operation = json!({
        "schema": RECEIPT_SCHEMA,
        "issuer": {"tool": "ultragoal", "self_law_state": "self_hosted"},
        "operation": "packet_verify",
        "candidate_digest": candidate,
        "status": "pass",
        "claim_ceiling": "supports_update_goal_eligibility",
        "blocked_claim_classes": [],
        "failure": null,
        "evidence_graph": super::graph::control(&candidate, "packet_verify", false)
    });
    let pass_failures =
        same_candidate_pass_failures(&wrong_operation, &candidate, "update_goal_eligibility");
    assert!(
        pass_failures.iter().any(|failure| failure
            == "cli_control_plane_receipt_wrong_operation:update_goal_eligibility"),
        "{pass_failures:?}"
    );

    let weak_failure = json!({
        "schema": RECEIPT_SCHEMA,
        "issuer": {"tool": "ultragoal", "self_law_state": "transition_only"},
        "operation": "update_goal_eligibility",
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
            "id": "manual_blocker",
            "law_id": "cli-control-plane-authority"
        },
        "evidence_graph": super::graph::control(&candidate, "update_goal_eligibility", true)
    });
    let fail_closed =
        same_candidate_fail_closed_failures(&weak_failure, &candidate, "update_goal_eligibility");
    assert!(
        fail_closed
            .iter()
            .any(|failure| failure == "cli_control_plane_receipt_failure_not_evidence_bound"),
        "{fail_closed:?}"
    );
}

#[test]
fn receipt_surface_rejects_malformed_or_mismatched_evidence_graphs() {
    let candidate = crate::self_tests::boundaries::workspace_fixtures::sha('e');
    let base = json!({
        "schema": RECEIPT_SCHEMA,
        "issuer": {"tool": "ultragoal", "self_law_state": "self_hosted"},
        "operation": "update_goal_eligibility",
        "candidate_digest": candidate,
        "status": "pass",
        "claim_ceiling": "supports_update_goal_eligibility",
        "blocked_claim_classes": [],
        "failure": null,
        "evidence_graph": super::graph::control(&candidate, "update_goal_eligibility", false)
    });

    let mut wrong_mode = base.clone();
    wrong_mode["evidence_graph"]["evaluation_mode"] = json!("test_constructor_not_authoritative");
    let failures = surface_value_failures(&wrong_mode);
    assert!(
        failures
            .iter()
            .any(|failure| failure == "cli_control_plane_receipt_evidence_graph_not_production")
    );

    let mut wrong_candidate = base.clone();
    wrong_candidate["evidence_graph"]["candidate_digest"] =
        json!(crate::self_tests::boundaries::workspace_fixtures::sha('f'));
    let failures = surface_value_failures(&wrong_candidate);
    assert!(failures
        .iter()
        .any(|failure| failure == "cli_control_plane_receipt_evidence_graph_candidate_mismatch"));

    let mut wrong_operation = base.clone();
    wrong_operation["evidence_graph"]["operation"] = json!("packet_verify");
    let failures = surface_value_failures(&wrong_operation);
    assert!(failures
        .iter()
        .any(|failure| failure == "cli_control_plane_receipt_evidence_graph_operation_mismatch"));

    let mut missing_items = base;
    missing_items["evidence_graph"]
        .as_object_mut()
        .expect("graph")
        .remove("items");
    let failures = surface_value_failures(&missing_items);
    assert!(
        failures
            .iter()
            .any(|failure| failure == "cli_control_plane_receipt_evidence_graph_missing_items")
    );
}

#[test]
fn pass_receipts_require_every_production_graph_label() {
    let candidate = crate::self_tests::boundaries::workspace_fixtures::sha('f');
    let mut value = json!({
        "schema": RECEIPT_SCHEMA,
        "issuer": {"tool": "ultragoal", "self_law_state": "self_hosted"},
        "operation": "update_goal_eligibility",
        "candidate_digest": candidate,
        "status": "pass",
        "claim_ceiling": "supports_update_goal_eligibility",
        "blocked_claim_classes": [],
        "failure": null,
        "evidence_graph": super::graph::control(&candidate, "update_goal_eligibility", false)
    });
    value["evidence_graph"]["items"]
        .as_array_mut()
        .expect("items")
        .retain(|item| {
            item.get("label").and_then(serde_json::Value::as_str)
                != Some("transactional_finalization")
        });

    let failures = same_candidate_pass_failures(&value, &candidate, "update_goal_eligibility");
    assert!(failures.iter().any(|failure| failure
        == "cli_control_plane_receipt_evidence_graph_missing_label:transactional_finalization"));
}
