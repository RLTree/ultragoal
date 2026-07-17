use super::cohesion_target_repository::{
    audit_product_cohesion, command_args_for_product_cohesion_dispatch, product_cohesion_detail,
    product_cohesion_repository_from_fixture, read_product_journey_receipt,
    write_human_review_exception_receipt, write_product_journey_receipt,
};
use serde_json::{Value, json};

#[test]
fn product_cohesion_audit_reports_missing_marker_and_malformed_receipt() {
    let empty = crate::self_tests::boundaries::workspace_fixtures::temp_root("product-empty");
    std::fs::create_dir_all(&empty).expect("empty target");
    let optional = audit_product_cohesion(&empty, false);
    assert_eq!(optional["status"], "not_applicable");
    assert!(product_cohesion_detail(&optional).contains("not requested"));
    let required = audit_product_cohesion(&empty, true);
    assert_eq!(required["status"], "blocked");
    assert!(product_cohesion_detail(&required).contains("docs/product-cohesion.md"));
    std::fs::remove_dir_all(empty).expect("cleanup empty");

    let no_marker = product_cohesion_repository_from_fixture(
        "product-no-marker",
        "fixtures/target-repo/valid-product-cohesion",
    );
    let script = no_marker.join("scripts/check");
    let text = std::fs::read_to_string(&script).expect("script");
    std::fs::write(
        &script,
        text.replace("echo \"harness-check:product-cohesion pass\"\n", ""),
    )
    .expect("remove marker");
    let marker_row = audit_product_cohesion(&no_marker, true);
    assert_eq!(marker_row["status"], "blocked");
    assert!(product_cohesion_detail(&marker_row).contains("gate marker"));
    std::fs::remove_dir_all(no_marker).expect("cleanup marker");

    let malformed = product_cohesion_repository_from_fixture(
        "product-malformed",
        "fixtures/target-repo/valid-product-cohesion",
    );
    std::fs::write(
        malformed.join("validation_artifacts/product-cohesion/journey-receipt.json"),
        "{",
    )
    .expect("malformed journey");
    let malformed_row = audit_product_cohesion(&malformed, true);
    assert_eq!(malformed_row["status"], "blocked");
    assert!(product_cohesion_detail(&malformed_row).contains("journey receipt malformed"));
    std::fs::remove_dir_all(malformed).expect("cleanup malformed");
}

#[test]
fn product_cohesion_human_attention_exception_branches_are_specific() {
    let no_exception = product_cohesion_repository_from_fixture(
        "product-no-exception",
        "fixtures/target-repo/valid-product-cohesion",
    );
    let mut journey = read_product_journey_receipt(&no_exception);
    journey["human_attention_policy"]["expected_interruption_rate"] = json!("frequent");
    journey["human_attention_policy"]["human_review_queue_exception"] = Value::Null;
    write_product_journey_receipt(&no_exception, &journey);
    let row = audit_product_cohesion(&no_exception, true);
    assert!(product_cohesion_detail(&row).contains("requires human review queue exception"));
    std::fs::remove_dir_all(no_exception).expect("cleanup no exception");

    let exception = product_cohesion_repository_from_fixture(
        "product-exception-branches",
        "fixtures/target-repo/red/exception-product-cohesion",
    );
    let schema_row = audit_product_cohesion(&exception, true);
    assert!(product_cohesion_detail(&schema_row).contains("evidence schema mismatch"));

    let surface_id = read_product_journey_receipt(&exception)["product_surface_id"].clone();
    write_human_review_exception_receipt(
        &exception,
        json!({
            "schema": "harness-ultragoal.human-review-queue-exception.v1",
            "status": "fail",
            "product_surface_id": surface_id
        }),
    );
    let status_row = audit_product_cohesion(&exception, true);
    assert!(product_cohesion_detail(&status_row).contains("authority receipt status not pass"));

    write_human_review_exception_receipt(
        &exception,
        json!({
            "schema": "harness-ultragoal.human-review-queue-exception.v1",
            "status": "pass",
            "product_surface_id": "other-surface"
        }),
    );
    let mismatch_row = audit_product_cohesion(&exception, true);
    assert!(product_cohesion_detail(&mismatch_row).contains("product surface mismatch"));

    write_human_review_exception_receipt(
        &exception,
        json!({
            "schema": "harness-ultragoal.human-review-queue-exception.v1",
            "status": "pass",
            "product_surface_id": surface_id,
            "intrinsic_human_decision": "short"
        }),
    );
    let field_row = audit_product_cohesion(&exception, true);
    assert!(product_cohesion_detail(&field_row).contains("missing substantive"));
    std::fs::remove_dir_all(exception).expect("cleanup exception");
}

#[test]
fn command_dispatch_returns_typed_exit_codes_for_fail_closed_paths() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let control_root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("control-dispatch");
    std::fs::create_dir_all(&control_root).expect("control dir");
    std::fs::write(
        control_root.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&json!({"version":"0.0.0-test","resources":[]}))
            .expect("control manifest"),
    )
    .expect("write control manifest");
    let control_receipt =
        std::path::PathBuf::from("validation_artifacts/cli/update-goal-eligibility.json");
    let control_code =
        crate::command_run::run_with_exit_code(command_args_for_product_cohesion_dispatch(
            control_root.clone(),
            &[
                "update-goal",
                "eligibility",
                "--receipt",
                control_receipt.to_str().expect("control path"),
            ],
        ))
        .expect("control command");
    assert_eq!(control_code, 1);
    assert_eq!(
        crate::json_boundary::read_json(&control_root.join(&control_receipt))
            .expect("control receipt")["status"],
        "fail"
    );

    let performance_receipt = std::path::PathBuf::from(
        "validation_artifacts/performance/performance-dispatch/performance.json",
    );
    let performance_code =
        crate::command_run::run_with_exit_code(command_args_for_product_cohesion_dispatch(
            root.clone(),
            &[
                "self",
                "performance",
                "prove",
                "--receipt",
                performance_receipt.to_str().expect("performance path"),
            ],
        ))
        .expect("performance command");
    assert_eq!(performance_code, 1);
    let performance_value = crate::json_boundary::read_json(&root.join(&performance_receipt))
        .expect("performance receipt");
    assert_eq!(performance_value["status"], "fail");
    let failure_id = performance_value["failure"]["id"].as_str().unwrap_or("");
    assert!(
        [
            "cli_performance_missing_node_speed_proof",
            "cli_performance_node_speed_proof_failed"
        ]
        .contains(&failure_id),
        "performance failure id {failure_id}"
    );

    let review_code =
        crate::command_run::run_with_exit_code(command_args_for_product_cohesion_dispatch(
            root.clone(),
            &[
                "review-round",
                "verify",
                "--receipt",
                "fixtures/review-round/valid/review-round-receipt.json",
                "--validator-receipt",
                "fixtures/review-round/anchors/validator-receipt.json",
                "--review-target-receipt",
                "fixtures/review-round/anchors/review-target-receipt.json",
                "--archive-receipt",
                "fixtures/review-round/anchors/archive-receipt.json",
            ],
        ))
        .expect("review command");
    assert_eq!(review_code, 1);
    std::fs::remove_dir_all(control_root).expect("cleanup control dispatch root");
    let _ = std::fs::remove_file(root.join(&performance_receipt));
}
