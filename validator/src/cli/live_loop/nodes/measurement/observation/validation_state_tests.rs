use super::super::full_command::FullCommandRun;
use super::*;
use crate::cli::live_loop::surfaces::surface_by_id;
use serde_json::{Value, json};
use std::path::PathBuf;

fn command() -> LiveLoopCommand {
    LiveLoopCommand {
        action: crate::cli::live_loop::LiveLoopAction::Measure,
        tier: "standard".to_string(),
        cache_mode: "verified-local".to_string(),
        jobs: None,
        receipt: PathBuf::from("validation_artifacts/observability/live-loop-node-timing.json"),
        node_id: Some("changed_files".to_string()),
        measure_all: false,
    }
}

fn actual_work(success: bool) -> FullCommandRun {
    FullCommandRun {
        exit_code: if success { 0 } else { 2 },
        status_success: success,
        launch_error: false,
        duration_ms: 11,
        stdout_digest: crate::digest::bytes(b"stdout"),
        stderr_digest: crate::digest::bytes(b"stderr"),
        executed_test_count: None,
        failure: Default::default(),
    }
}

fn prepared_root(label: &str) -> std::path::PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    std::fs::create_dir_all(&root).expect("root");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    root
}

#[test]
fn reconciliation_keeps_validation_result_independent_from_live_query_outcome() {
    let root = prepared_root("live-loop-reconcile-query-outcome");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let surface = surface_by_id("changed_files").expect("surface");

    let reconciliation = reconcile(&root, surface, &candidate, &command(), &actual_work(true));

    assert!(
        ["pass", "query_or_explain_reconciliation_failed"]
            .contains(&reconciliation.status.as_str()),
        "{}",
        reconciliation.status
    );
    assert_eq!(
        reconciliation.value["command_observation"]["status"],
        "pass"
    );
    assert!(
        ["pass", "fail"].contains(
            &reconciliation.value["logs_query"]["status"]
                .as_str()
                .unwrap_or_default()
        )
    );
    assert_eq!(
        reconciliation.value["claim_impact"],
        "source_local_live_loop_node_observation_only_not_speed_claim"
    );
    let failure = reconciliation.failure_summary();
    assert!(
        failure.failed_law.is_none()
            && failure.failed_check.is_none()
            && failure.why_failed.as_deref().unwrap_or("none") == "none"
            && failure.where_failed.as_deref().unwrap_or("none") == "none",
        "observability partial state must not replace the successful validation result: {:?}",
        failure.to_value()
    );
    std::fs::remove_dir_all(root).expect("cleanup unavailable backends");
}

#[test]
fn observation_pass_requires_logs_metrics_traces_and_explain_channels() {
    let root = prepared_root("live-loop-reconcile-synthetic-pass");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let surface = surface_by_id("changed_files").expect("surface");
    let mut roundtrip = |roundtrip: query::RoundtripQuery,
                         run_id: &str,
                         correlation_id: &str|
     -> Result<query::ObserveReceipt, String> {
        let kind = match roundtrip {
            query::RoundtripQuery::Logs => "logs",
            query::RoundtripQuery::Metrics => "metrics",
            query::RoundtripQuery::Traces => "traces",
            query::RoundtripQuery::ExplainFailure => "explain",
        };
        let value = if roundtrip == query::RoundtripQuery::ExplainFailure {
            json!({
                "status": "pass",
                "run_id": run_id,
                "correlation_id": correlation_id,
                "failure_class": "none",
                "claim_impact": "query_observation_only",
                "bounded_output_proof": "pass",
                "explanation": {
                    "query_evidence": {
                        "logs": {"status": "pass"},
                        "metrics": {"status": "pass"},
                        "traces": {"status": "pass"}
                    }
                }
            })
        } else {
            json!({
                "status": "pass",
                "run_id": run_id,
                "correlation_id": correlation_id,
                "bounded_output_status": "pass",
                "redaction_status": "pass",
                "row_count": 1,
                "result_digest": crate::digest::bytes(kind.as_bytes())
            })
        };
        Ok(query::ObserveReceipt {
            receipt: format!("validation_artifacts/observability/live-loop/commands/{kind}.json"),
            exit_code: 0,
            status: "pass".to_string(),
            duration_ms: 3,
            value,
        })
    };

    let report = test_reconciliation::reconcile_with_observe_roundtrip(
        &root,
        surface,
        &candidate,
        &command(),
        &actual_work(true),
        &mut roundtrip,
    )
    .expect("synthetic query reconciliation");

    assert_eq!(report.status, "pass");
    assert_eq!(report.value["slowest_roundtrip"]["duration_ms"], 3);
    assert_eq!(report.value["first_failed_roundtrip"], Value::Null);
    std::fs::remove_dir_all(root).expect("cleanup synthetic pass");
}
