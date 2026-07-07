use super::{
    LiveLoopAction, LiveLoopCommand,
    receipt::{claim_evaluation, failure_class, telemetry_status, where_failed},
};
use serde_json::json;
use std::path::PathBuf;

#[test]
fn status_projection_reports_pass_without_failure_fields() {
    let blocker = json!({
        "failure_class": "live_loop_telemetry_reconciliation_missing",
        "where_failed": "loop.measure.fmt_check.telemetry_reconciliation"
    });
    assert_eq!(failure_class("pass", &blocker), "none");
    assert_eq!(where_failed("pass", &blocker), "none");
    assert_eq!(
        failure_class("fail", &blocker),
        "live_loop_telemetry_reconciliation_missing"
    );
    assert_eq!(
        where_failed("fail", &blocker),
        "loop.measure.fmt_check.telemetry_reconciliation"
    );
}

#[test]
fn loop_receipt_claim_evaluation_names_pass_and_blocked_surfaces() {
    let command = LiveLoopCommand {
        action: LiveLoopAction::Run,
        tier: "hot".to_string(),
        cache_mode: "verified-local".to_string(),
        jobs: None,
        receipt: PathBuf::from("validation_artifacts/observability/live-loop-run.json"),
        node_id: None,
        measure_all: false,
    };
    let first_blocker = json!({
        "id": "coverage_prove",
        "why_failed": "coverage receipt is stale"
    });

    let pass = claim_evaluation("pass", &command, &first_blocker);
    assert_eq!(pass["claim_status"], "supported_source_local");
    assert_eq!(
        pass["product_behavior_observed"],
        "ultragoal loop run --tier hot --cache-mode verified-local"
    );
    assert!(
        pass["proof_surface"]
            .as_str()
            .expect("pass proof surface")
            .contains("executed or verified-cache timing proof")
    );
    assert!(
        pass["independent_reconciliation_surface"]
            .as_str()
            .expect("pass reconciliation")
            .contains("logs, metrics, traces")
    );

    let blocked = claim_evaluation("fail", &command, &first_blocker);
    assert_eq!(blocked["claim_status"], "blocked");
    assert!(
        blocked["proof_surface"]
            .as_str()
            .expect("blocked proof surface")
            .contains("no acceleration claim")
    );
    assert_eq!(blocked["first_blocker"]["id"], "coverage_prove");

    let partial = claim_evaluation("partial", &command, &first_blocker);
    assert_eq!(
        partial["claim_status"],
        "withheld_validation_result_available"
    );
    assert!(
        partial["proof_surface"]
            .as_str()
            .expect("partial proof surface")
            .contains("no acceleration claim")
    );
}

#[test]
fn partial_loop_status_maps_to_schema_valid_telemetry_blocked_status() {
    assert_eq!(telemetry_status("pass"), "pass");
    assert_eq!(telemetry_status("fail"), "fail");
    assert_eq!(telemetry_status("partial"), "blocked");
    assert_eq!(telemetry_status("unknown"), "blocked");
}
