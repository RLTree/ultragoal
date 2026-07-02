use super::*;
use crate::cli::observe::types::{ObserveCommand, ObserveOperation};
use serde_json::json;

fn command(operation: ObserveOperation) -> ObserveCommand {
    ObserveCommand {
        operation,
        receipt: None,
        query: None,
        run_id: None,
        correlation_id: None,
        claim_id: None,
        check_id: None,
        law_id: None,
        row_limit: 100,
        byte_limit: 4096,
        timeout_ms: 1000,
    }
}

#[test]
fn fitting_inventory_summary_names_first_blocker_and_family_counts() {
    let root = crate::self_tests::boundaries::support::temp_root("observe-telemetry-summary");
    let inventory = root.join("docs/generated/observability");
    std::fs::create_dir_all(&inventory).expect("inventory dir");
    crate::json_boundary::write_json(
        &inventory.join("command-inventory.json"),
        &json!({
            "fitting_control_board": {
                "status": "fail",
                "first_incomplete": {
                    "family": "commands",
                    "id": "coverage.prove",
                    "fitting_status": "partial",
                    "next_unfitted_surface": "traces_query"
                },
                "families": {
                    "commands": {"total": 2, "fitted": 1, "partially_fitted": 1, "unfitted": 0},
                    "claims": {"total": 1, "fitted": 0, "partially_fitted": 0, "unfitted": 1}
                }
            }
        }),
    )
    .expect("inventory");

    let summary = fitting_inventory_failure_summary(
        &root,
        &["observability_command_fitting_query_not_current:coverage.prove".to_string()],
    );

    assert!(summary.contains("control_board_first_family=commands"));
    assert!(summary.contains("control_board_first_incomplete=coverage.prove"));
    assert!(summary.contains("claims=1/0/0/1"));
    assert_eq!(family_counts(&json!({})), "unavailable");
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn claim_repairs_cover_current_failure_classes() {
    assert_eq!(
        claims::next_repair_for(
            ObserveOperation::Prove,
            "fail",
            Some("first_failure=observability_command_fitting_query_not_current")
        ),
        "refresh the first same-candidate query proof named in why_failed: rerun the target command, query logs metrics traces for that run, then rerun observe prove"
    );
    assert_eq!(
        claims::next_repair_for(
            ObserveOperation::Prove,
            "fail",
            Some("first_failure=observability_command_fitting_receipt_missing")
        ),
        "refresh the first command receipt named in why_failed on the current candidate, then query logs metrics traces and rerun observe prove"
    );
    assert_eq!(
        claims::next_repair_for(
            ObserveOperation::ExplainFailure,
            "fail",
            Some("observed telemetry candidate digest mismatch")
        ),
        "rerun target command on the current candidate before claiming observability fit"
    );
    assert_eq!(
        claims::next_repair_for(command(ObserveOperation::LogsQuery).operation, "pass", None),
        "keep receipt same-candidate and rerun source audit before any readiness claim"
    );
}

#[test]
fn metric_summary_rejects_high_cardinality_labels_and_ignores_bad_rows() {
    let rows = vec![
        json!({"body": "not-json"}),
        json!({"no_body": true}),
        json!({"body": json!({
            "data": {"result": [
                {
                    "metric": {
                        "__name__": "ultragoal_command_total",
                        "operation": "coverage.prove",
                        "status": "fail",
                        "failure_class": "coverage_prove_failure",
                        "saturation_status": "queue_pressure",
                        "run_id": "run-high-cardinality"
                    },
                    "value": [10, "2"]
                },
                {
                    "metric": {
                        "__name__": "ultragoal_command_duration_ms",
                        "operation": "coverage.prove",
                        "status": "fail",
                        "failure_class": "coverage_prove_failure"
                    },
                    "value": [10.6, "42.4"]
                },
                {
                    "metric": {
                        "__name__": "ultragoal_command_task_count",
                        "operation": "coverage.prove",
                        "status": "fail",
                        "failure_class": "coverage_prove_failure"
                    },
                    "value": ["11", "3"]
                },
                {
                    "metric": {
                        "__name__": "ultragoal_command_queue_depth",
                        "operation": "coverage.prove",
                        "status": "fail",
                        "failure_class": "coverage_prove_failure",
                        "saturation_status": "queue_pressure_after_join"
                    },
                    "value": [12, "7"]
                }
            ]}
        }).to_string()}),
    ];

    let summary = metric_summary(&rows);

    assert_eq!(summary["operation"], "coverage.prove");
    assert_eq!(summary["traffic_count"], 2);
    assert_eq!(summary["error_count"], 2);
    assert_eq!(summary["failure_class"], "coverage_prove_failure");
    assert_eq!(summary["latency_ms"], 42);
    assert_eq!(summary["task_count"], 3);
    assert_eq!(summary["queue_depth"], 7);
    assert_eq!(summary["latest_sample_unix"], 12);
    assert_eq!(
        summary["saturation_status"],
        "queue_pressure_after_join;queue_depth=7"
    );
    assert_eq!(summary["high_cardinality_labels"], "fail");
}

#[test]
fn telemetry_receipt_text_is_required_for_query_identity() {
    assert_eq!(
        query_receipt_text_for_test(&json!({"run_id":"run-required"}), "run_id").expect("run id"),
        "run-required"
    );
    let error =
        query_receipt_text_for_test(&json!({}), "run_id").expect_err("missing run id fails");
    assert_eq!(error, "observability telemetry receipt missing run_id");
}
