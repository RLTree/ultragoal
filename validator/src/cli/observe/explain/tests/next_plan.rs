use serde_json::json;
use std::fs;

const BLOCKER: &str = "HCT-OBSERVE successor catalog unavailable/not adopted";

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
    fs::write(
        root.join("docs/generated/observability/command-inventory.json"),
        "SECRET_CANARY",
    )
    .expect("static catalog bait");
    let command = crate::cli::observe::parse(&[
        "observe".to_string(),
        "explain".to_string(),
        "--next".to_string(),
        "--receipt".to_string(),
        "validation_artifacts/observability/observe-explain-next.json".to_string(),
    ])
    .expect("parse")
    .expect("observe explain next");

    let code = crate::cli::observe::run(&root, &command).expect("observe explain next");
    assert_eq!(code, 1);
    let receipt = crate::json_boundary::read_json(
        &root.join("validation_artifacts/observability/observe-explain-next.json"),
    )
    .expect("explain next receipt");

    assert_eq!(receipt["status"], "fail");
    assert_eq!(receipt["target_row"], "HCT-OBSERVE");
    assert_eq!(receipt["target_family"], "successor_catalog");
    assert_eq!(receipt["why_failed"], BLOCKER);
    assert_eq!(receipt["supported_claims"], json!([]));
    assert_eq!(receipt["required_query_commands"], json!([]));
    assert_eq!(receipt["explanation"]["fallback_used"], false);
    assert_eq!(
        receipt["explanation"]["row"]["validator_check_id"],
        "not_adopted"
    );
    assert!(
        receipt["blocked_claims"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item == "update_goal_eligibility")
    );
    assert!(!receipt.to_string().contains("SECRET_CANARY"));
    assert!(
        !root
            .join("validation_artifacts/observability/spool")
            .exists()
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
        "--receipt".to_string(),
        "validation_artifacts/observability/observe-explain-next.json".to_string(),
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
    let command = crate::cli::observe::parse(&[
        "observe".to_string(),
        "explain".to_string(),
        "--next".to_string(),
        "--receipt".to_string(),
        "validation_artifacts/observability/observe-explain-next.json".to_string(),
    ])
    .expect("parse")
    .expect("observe explain next");

    let code = crate::cli::observe::run(&root, &command).expect("observe explain next");
    assert_eq!(code, 1);
    let receipt = crate::json_boundary::read_json(
        &root.join("validation_artifacts/observability/observe-explain-next.json"),
    )
    .expect("explain next receipt");

    assert_eq!(receipt["status"], "fail");
    assert_eq!(receipt["target_row"], "HCT-OBSERVE");
    assert_eq!(receipt["target_family"], "successor_catalog");
    assert_eq!(receipt["why_failed"], BLOCKER);
    assert_eq!(receipt["supported_claims"], json!([]));
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
    assert!(
        !root
            .join("validation_artifacts/observability/spool")
            .exists()
    );
    fs::remove_dir_all(root).expect("cleanup explain next defaults");
}

#[test]
fn explain_next_explicit_output_ignores_blocked_telemetry_spool() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("observe-explain-next-spool");
    fs::create_dir_all(root.join("docs/generated/observability")).expect("inventory dir");
    fs::create_dir_all(root.join("validation_artifacts/observability")).expect("telemetry dir");
    let spool = root.join("validation_artifacts/observability/spool");
    fs::write(&spool, "not a directory SECRET_CANARY").expect("spool sentinel");
    fs::write(root.join("owned.txt"), "owned").expect("owned");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["owned.txt"]}),
    )
    .expect("manifest");
    fs::write(
        root.join("docs/generated/observability/command-inventory.json"),
        [0xff, 0xfe],
    )
    .expect("static catalog bait");
    let command = crate::cli::observe::parse(&[
        "observe".to_string(),
        "explain".to_string(),
        "--next".to_string(),
        "--receipt".to_string(),
        "validation_artifacts/observability/observe-explain-next.json".to_string(),
    ])
    .expect("parse")
    .expect("observe explain next");

    assert_eq!(
        crate::cli::observe::run(&root, &command).expect("explain next"),
        1
    );
    assert_eq!(
        fs::read_to_string(&spool).expect("spool unchanged"),
        "not a directory SECRET_CANARY"
    );
    let receipt = crate::json_boundary::read_json(
        &root.join("validation_artifacts/observability/observe-explain-next.json"),
    )
    .expect("explain next receipt");
    assert_eq!(receipt["status"], "fail");
    assert_eq!(receipt["why_failed"], BLOCKER);
    assert_eq!(receipt["supported_claims"], json!([]));
    assert!(!receipt.to_string().contains("SECRET_CANARY"));
    fs::remove_dir_all(root).expect("cleanup explain next spool");
}
