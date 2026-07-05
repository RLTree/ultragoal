use serde_json::json;
use std::fs;

#[test]
fn explain_next_reports_control_board_repair_plan() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("observe-explain-next");
    fs::create_dir_all(root.join("docs/generated/observability")).expect("inventory dir");
    fs::write(root.join("owned.txt"), "owned").expect("owned");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["owned.txt"]}),
    )
    .expect("manifest");
    crate::json_boundary::write_json(
        &root.join("docs/generated/observability/command-inventory.json"),
        &json!({
            "observability_control_board": {
                "status": "blocked",
                "first_incomplete": {
                    "family": "commands",
                    "id": "install audit",
                    "observability_status": "partially_observable",
                    "next_unobservable_surface": "red/green/tamper fixture proof"
                }
            },
            "command_observability_inventory": {
                "install audit": {
                    "observability_status": "partially_observable",
                    "missing_surfaces": [
                        "red fixture proof",
                        "green fixture proof",
                        "tamper fixture proof"
                    ],
                    "validator_check_id": "install-audit-observability-binding",
                    "current_owner_surface": "command:install audit",
                    "receipt_paths": ["validation_artifacts/cli/install-audit-receipt.json"],
                    "same_candidate_query_proof_paths": [
                        "validation_artifacts/observability/install-audit-logs-query.json",
                        "validation_artifacts/observability/install-audit-metrics-query.json",
                        "validation_artifacts/observability/install-audit-traces-query.json"
                    ],
                    "focused_tests": [
                        "package_surface_run_writes_typed_install_observability"
                    ],
                    "claim_impact": "blocks_install_parity_readiness_release_completion_update_goal"
                }
            }
        }),
    )
    .expect("inventory");
    let command = crate::cli::observe::parse(&[
        "observe".to_string(),
        "explain".to_string(),
        "--next".to_string(),
    ])
    .expect("parse")
    .expect("observe explain next");

    let code = crate::cli::observe::run(&root, &command).expect("observe explain next");
    assert_eq!(code, 0);
    let receipt = crate::json_boundary::read_json(
        &root.join("validation_artifacts/observability/observe-explain-next.json"),
    )
    .expect("explain next receipt");

    assert_eq!(receipt["status"], "pass");
    assert_eq!(receipt["target_row"], "install audit");
    assert_eq!(receipt["target_family"], "commands");
    assert_eq!(
        receipt["next_unobservable_surface"],
        "red/green/tamper fixture proof"
    );
    assert_eq!(receipt["explanation"]["fallback_used"], false);
    assert!(
        receipt["next_repair"]
            .as_str()
            .unwrap()
            .contains("red/green/tamper fixture proof")
    );
    assert!(
        receipt["explanation"]["smallest_repair"]
            .as_str()
            .unwrap()
            .contains("ultragoal observe fit --command \"install audit\"")
    );
    assert_eq!(
        receipt["explanation"]["row"]["validator_check_id"],
        "install-audit-observability-binding"
    );
    assert!(
        receipt["required_query_commands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str().unwrap().contains("observe logs query"))
    );
    assert!(
        receipt["blocked_claims"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item == "update_goal_eligibility")
    );
    fs::remove_dir_all(root).expect("cleanup explain next");
}

#[test]
fn explain_next_rejects_non_package_root_before_planning() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("observe-explain-next-root");
    fs::create_dir_all(&root).expect("root");
    let command = crate::cli::observe::parse(&[
        "observe".to_string(),
        "explain".to_string(),
        "--next".to_string(),
    ])
    .expect("parse")
    .expect("observe explain next");

    let error = crate::cli::observe::run(&root, &command)
        .expect_err("explain next rejects roots without package authority");
    assert!(error.contains("plugin-manifest-draft.json"), "{error}");
    fs::remove_dir_all(root).expect("cleanup explain next bad root");
}

#[test]
fn explain_next_defaults_missing_inventory_without_claiming_observability() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "observe-explain-next-defaults",
    );
    fs::create_dir_all(root.join("docs/generated/observability")).expect("inventory dir");
    fs::write(root.join("owned.txt"), "owned").expect("owned");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["owned.txt"]}),
    )
    .expect("manifest");
    crate::json_boundary::write_json(
        &root.join("docs/generated/observability/command-inventory.json"),
        &json!({
            "observability_control_board": {
                "status": "blocked",
                "first_incomplete": {
                    "family": "unknown_family",
                    "id": "unknown surface",
                    "observability_status": "unobservable",
                    "next_unobservable_surface": "same-candidate query proof"
                }
            }
        }),
    )
    .expect("inventory");
    let command = crate::cli::observe::parse(&[
        "observe".to_string(),
        "explain".to_string(),
        "--next".to_string(),
    ])
    .expect("parse")
    .expect("observe explain next");

    let code = crate::cli::observe::run(&root, &command).expect("observe explain next");
    assert_eq!(code, 0);
    let receipt = crate::json_boundary::read_json(
        &root.join("validation_artifacts/observability/observe-explain-next.json"),
    )
    .expect("explain next receipt");

    assert_eq!(receipt["target_row"], "unknown surface");
    assert_eq!(receipt["target_family"], "unknown_family");
    assert_eq!(receipt["owner_surface"], "unknown_family");
    assert_eq!(receipt["missing_proof_class"], "same-candidate query proof");
    assert_eq!(
        receipt["claim_impact"],
        "blocks_observability_product_closure_readiness_release_completion_update_goal"
    );
    assert_eq!(receipt["explanation"]["implicated_paths"], json!([]));
    assert_eq!(receipt["explanation"]["row"]["focused_tests"], json!([]));
    assert_eq!(receipt["explanation"]["row"]["receipt_paths"], json!([]));
    assert_eq!(
        receipt["explanation"]["row"]["same_candidate_query_proof_paths"],
        json!([])
    );
    assert!(
        receipt["blocked_claims"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item == "observability_product_closure")
    );
    fs::remove_dir_all(root).expect("cleanup explain next defaults");
}

#[test]
fn explain_next_fails_when_telemetry_spool_cannot_be_written() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("observe-explain-next-spool");
    fs::create_dir_all(root.join("docs/generated/observability")).expect("inventory dir");
    fs::create_dir_all(root.join("validation_artifacts/observability")).expect("telemetry dir");
    fs::write(
        root.join("validation_artifacts/observability/spool"),
        "not a directory",
    )
    .expect("spool blocker");
    fs::write(root.join("owned.txt"), "owned").expect("owned");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["owned.txt"]}),
    )
    .expect("manifest");
    crate::json_boundary::write_json(
        &root.join("docs/generated/observability/command-inventory.json"),
        &json!({"observability_control_board": {"status": "blocked"}}),
    )
    .expect("inventory");
    let command = crate::cli::observe::parse(&[
        "observe".to_string(),
        "explain".to_string(),
        "--next".to_string(),
    ])
    .expect("parse")
    .expect("observe explain next");

    let error = crate::cli::observe::run(&root, &command)
        .expect_err("explain next fails closed when telemetry cannot be spooled");
    assert!(
        error.contains("validation_artifacts/observability/spool"),
        "{error}"
    );
    fs::remove_dir_all(root).expect("cleanup explain next spool");
}
