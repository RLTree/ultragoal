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

fn write_text(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, text).expect("write text");
}

#[test]
fn receipt_blocks_claims_and_records_required_evidence() {
    let root = repo_root();
    let registry = ControlCommand {
        operation: ControlOperation::RegistryProbe,
        receipt: None,
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
    };
    let value = receipt(&root, &update_goal).expect("update-goal receipt builds");
    assert!(
        value["required_evidence"]
            .as_array()
            .expect("required evidence")
            .iter()
            .any(|item| item.as_str() == Some("all_89_gates_and_100_stop_conditions_pass"))
    );
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
    };
    assert_eq!(run(&root, &command).expect("run writes receipt"), 1);
    let value = crate::json_boundary::read_json(&path).expect("read receipt");
    assert_eq!(value["operation"], "packet_verify");
    assert_eq!(value["claim_ceiling"], "withheld_or_blocked");

    let no_write = ControlCommand {
        operation: ControlOperation::FixturesAll,
        receipt: None,
    };
    assert_eq!(run(&root, &no_write).expect("run prints receipt"), 1);
    std::fs::remove_dir_all(root).expect("cleanup cli control run");
}

#[test]
fn surface_validation_rejects_missing_authority_fields() {
    let mut value = json!({
        "schema": RECEIPT_SCHEMA,
        "issuer": {"tool": "ultragoal"},
        "operation": "packet_verify",
        "candidate_digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "blocked_claim_classes": ["completion"],
        "failure": {"law_id": "cli-self-law-compliance"}
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
    value["failure"]["law_id"] = json!("other");
    let failures = surface_value_failures(&value);
    for expected in [
        "cli_control_plane_receipt_wrong_schema",
        "cli_control_plane_receipt_wrong_issuer",
        "cli_control_plane_receipt_missing_operation",
        "cli_control_plane_receipt_missing_candidate_digest",
        "cli_control_plane_receipt_missing_blocked_claims",
        "cli_control_plane_receipt_missing_self_law_failure",
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
        "failure": {"law_id": "cli-self-law-compliance"}
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
        "failure": null
    });
    assert!(
        same_candidate_pass_failures(&pass, &current_candidate, "update_goal_eligibility")
            .is_empty()
    );
}

#[test]
fn cli_control_plane_audit_rejects_missing_schema_and_invalid_receipts() {
    let root = crate::self_tests::boundaries::support::temp_root("cli-control-plane-audit");
    write_text(
        &root.join("validator/Cargo.toml"),
        "[[bin]]\nname = \"ultragoal\"\n[[bin]]\nname = \"ultragoal-validator\"\n",
    );
    write_text(&root.join("validator/src/cli/control/plane.rs"), "source");
    write_text(
        &root.join("validator/src/cli/control/plane/types.rs"),
        "types",
    );
    write_json(
        &root.join("schemas/cli-control-plane-receipt.schema.json"),
        &json!({}),
    );
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[
            {"path":"validator/src/cli/control/plane.rs"},
            {"path":"validator/src/cli/control/plane/types.rs"},
            {"path":"schemas/cli-control-plane-receipt.schema.json"}
        ]}),
    );
    write_json(
        &root.join("schemas/schema-catalog.json"),
        &json!({"schemas":[]}),
    );
    write_json(
        &root.join("templates/agent-standards/enforcement.json"),
        &json!({"rows":[]}),
    );
    write_json(
        &root.join("docs/source-obligation-matrix.json"),
        &json!({"obligations":[]}),
    );
    write_json(
        &root.join("docs/foundational-law-traceability.json"),
        &json!({"entries":[]}),
    );
    write_json(&root.join("templates/RED_FIXTURES.json"), &json!([]));
    write_json(
        &root.join("validation_artifacts/cli/update-goal-eligibility.json"),
        &json!({"schema":"wrong","issuer":{"tool":"manual"}}),
    );
    write_json(
        &root.join("validation_artifacts/cli/self-law-receipt.json"),
        &json!({"schema":"wrong","issuer":{"tool":"manual"}}),
    );
    let failures = crate::audit::cli::control_plane::authority::package_failures(&root);
    assert!(
        failures.contains(&"cli_control_plane_schema_catalog_missing_receipt_schema".to_string())
    );
    assert!(failures.iter().any(|failure| failure.contains(
        "validation_artifacts/cli/update-goal-eligibility.json: cli_control_plane_receipt_wrong_schema"
    )));
    assert!(failures.iter().any(|failure| failure.contains(
        "validation_artifacts/cli/self-law-receipt.json: cli_control_plane_receipt_wrong_issuer"
    )));
    std::fs::remove_dir_all(root).expect("cleanup cli control audit");
}
