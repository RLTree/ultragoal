use crate::cli::control::plane::types::ControlOperation;
use crate::cli::control::plane::{
    ControlCommand, RECEIPT_SCHEMA, receipt,
    receipt::{same_candidate_pass_failures, surface_value_failures},
    run,
};
use serde_json::json;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("validator has repo parent")
        .to_path_buf()
}

fn write_json(path: &Path, value: &serde_json::Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

#[test]
fn receipt_blocks_claims_and_records_required_evidence() {
    let root = repo_root();
    let registry = ControlCommand {
        operation: ControlOperation::RegistryProbe,
        receipt: None,
        surface_root: None,
    };
    let value = receipt(&root, &registry).expect("receipt builds");
    assert_eq!(value["schema"], RECEIPT_SCHEMA);
    assert_eq!(value["status"], "fail");
    assert_eq!(value["operation"], "registry_probe");
    assert_eq!(value["failure"]["law_id"], "cli-control-plane-authority");
    assert!(
        value["blocked_claim_classes"]
            .as_array()
            .expect("claims")
            .iter()
            .any(|claim| claim.as_str() == Some("app_registry_or_reviewer_exposure"))
    );
    assert!(surface_value_failures(&value).is_empty());

    let update_goal = ControlCommand {
        operation: ControlOperation::SelfUpdateGoalEligibility,
        receipt: None,
        surface_root: None,
    };
    let value = receipt(&root, &update_goal).expect("update-goal receipt builds");
    assert!(
        value["required_evidence"]
            .as_array()
            .expect("required evidence")
            .iter()
            .any(|item| item.as_str()
                == Some(crate::cli::control::plane::emit::FULL_CONTRACT_EVIDENCE))
    );
}

#[test]
fn control_plane_schema_accepts_emitted_observability_fields() {
    let root = crate::self_tests::boundaries::support::temp_root("cli-control-schema-observe");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"version":"0.0.0-test","resources":[]}),
    );
    let path = root.join("validation_artifacts/cli/self-law-receipt.json");
    let command = ControlCommand {
        operation: ControlOperation::SelfUpdateGoalEligibility,
        receipt: Some(path.clone()),
        surface_root: None,
    };
    assert_eq!(run(&root, &command).expect("control run writes receipt"), 1);
    let value = crate::json_boundary::read_json(&path).expect("control receipt");
    assert!(value.get("cache_mode").is_some());
    assert!(value.get("receipt_observability_binding").is_some());
    assert!(value.get("observability").is_some());

    let store = crate::schema_catalog::load(&repo_root());
    let errors = crate::schema_catalog::schema_errors(
        &store,
        "cli-control-plane-receipt.schema.json",
        &value,
    );
    assert!(errors.is_empty(), "{errors:?}");
    std::fs::remove_dir_all(root).expect("cleanup control schema observe");
}

#[test]
fn run_writes_and_prints_fail_closed_receipts() {
    let root = crate::self_tests::boundaries::support::temp_root("cli-control-receipt-run");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"version":"0.0.0-test","resources":[]}),
    );
    let path = root.join("validation_artifacts/cli/packet-verify-receipt.json");
    let command = ControlCommand {
        operation: ControlOperation::PacketVerify,
        receipt: Some(path.clone()),
        surface_root: None,
    };
    assert_eq!(run(&root, &command).expect("run writes receipt"), 1);
    let value = crate::json_boundary::read_json(&path).expect("read receipt");
    assert_eq!(value["operation"], "packet_verify");
    assert_eq!(value["claim_ceiling"], "withheld_or_blocked");

    let no_write = ControlCommand {
        operation: ControlOperation::FixturesAll,
        receipt: None,
        surface_root: None,
    };
    let missing_receipt = run(&root, &no_write).expect_err("run requires receipt path");
    assert!(missing_receipt.contains("missing required argument --receipt"));
    std::fs::remove_dir_all(root).expect("cleanup cli control run");
}

#[test]
fn production_constructor_does_not_pass_from_empty_evidence_without_green_graph() {
    let root = crate::self_tests::boundaries::support::temp_root("cli-production-no-green");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"version":"0.0.0-test","resources":[]}),
    );
    let candidate = crate::self_tests::boundaries::support::sha('c');
    let value = crate::cli::control::plane::emit::receipt_from_production_evidence(
        &root,
        candidate.clone(),
        ControlOperation::UpdateGoalEligibility,
        Vec::new(),
    );
    assert_eq!(value["status"], "fail");
    assert_eq!(value["candidate_digest"], candidate);
    assert_eq!(value["issuer"]["self_law_state"], "transition_only");
    assert_eq!(value["claim_ceiling"], "withheld_or_blocked");
    assert_eq!(
        value["evidence_graph"]["evaluation_mode"],
        "production_dereferenced"
    );
    std::fs::remove_dir_all(root).expect("cleanup cli production root");
}

#[test]
fn surface_validation_rejects_missing_authority_fields() {
    let mut value = json!({
        "schema": RECEIPT_SCHEMA,
        "issuer": {"tool": "ultragoal"},
        "operation": "packet_verify",
        "candidate_digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "blocked_claim_classes": ["completion"],
        "failure": {"law_id": "cli-self-law-compliance"},
        "evidence_graph": super::graph::control(
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "packet_verify",
            true
        )
    });
    assert!(surface_value_failures(&value).is_empty());

    value["schema"] = json!("wrong");
    value["issuer"]["tool"] = json!("manual");
    value.as_object_mut().expect("object").remove("operation");
    value
        .as_object_mut()
        .expect("object")
        .remove("candidate_digest");
    value
        .as_object_mut()
        .expect("object")
        .remove("blocked_claim_classes");
    value
        .as_object_mut()
        .expect("object")
        .remove("evidence_graph");
    value["failure"]["law_id"] = json!("other");
    let failures = surface_value_failures(&value);
    for expected in [
        "cli_control_plane_receipt_wrong_schema",
        "cli_control_plane_receipt_wrong_issuer",
        "cli_control_plane_receipt_missing_operation",
        "cli_control_plane_receipt_missing_candidate_digest",
        "cli_control_plane_receipt_missing_blocked_claims",
        "cli_control_plane_receipt_missing_self_law_failure",
        "cli_control_plane_receipt_missing_evidence_graph",
    ] {
        assert!(failures.iter().any(|failure| failure == expected));
    }
}

#[test]
fn strict_surface_validation_rejects_transition_only_or_wrong_candidate_receipts() {
    let stale_candidate = crate::self_tests::boundaries::support::sha('a');
    let current_candidate = crate::self_tests::boundaries::support::sha('b');
    let transition = json!({
        "schema": RECEIPT_SCHEMA,
        "issuer": {"tool": "ultragoal", "self_law_state": "transition_only"},
        "operation": "update_goal_eligibility",
        "candidate_digest": stale_candidate,
        "status": "fail",
        "claim_ceiling": "withheld_or_blocked",
        "blocked_claim_classes": ["completion", "update_goal_eligibility"],
        "failure": {"law_id": "cli-self-law-compliance"},
        "evidence_graph": super::graph::control(&stale_candidate, "update_goal_eligibility", true)
    });
    let failures =
        same_candidate_pass_failures(&transition, &current_candidate, "update_goal_eligibility");
    for expected in [
        "cli_control_plane_receipt_candidate_digest_mismatch",
        "cli_control_plane_receipt_not_pass",
        "cli_control_plane_receipt_not_self_hosted",
        "cli_control_plane_receipt_blocks_claims",
    ] {
        assert!(
            failures.iter().any(|failure| failure.contains(expected)),
            "{expected}: {failures:?}"
        );
    }

    let pass = json!({
        "schema": RECEIPT_SCHEMA,
        "issuer": {"tool": "ultragoal", "self_law_state": "self_hosted"},
        "operation": "update_goal_eligibility",
        "candidate_digest": current_candidate,
        "status": "pass",
        "claim_ceiling": "supports_update_goal_eligibility",
        "blocked_claim_classes": [],
        "failure": null,
        "evidence_graph": super::graph::control(&current_candidate, "update_goal_eligibility", false)
    });
    assert!(
        same_candidate_pass_failures(&pass, &current_candidate, "update_goal_eligibility")
            .is_empty()
    );
}
