use super::*;
use crate::cli::live_loop::surfaces::surface_by_id;
use crate::self_tests::boundaries::workspace_fixtures::temp_root;
use serde_json::json;
use std::path::{Path, PathBuf};

fn measure_command() -> LiveLoopCommand {
    LiveLoopCommand {
        action: crate::cli::live_loop::LiveLoopAction::Measure,
        tier: "hot".to_string(),
        cache_mode: "verified-local".to_string(),
        jobs: None,
        receipt: PathBuf::from("validation_artifacts/observability/live-loop-node-timing.json"),
        node_id: Some("changed_files".to_string()),
        measure_all: false,
    }
}

fn command_run(duration_ms: u64) -> FullCommandRun {
    FullCommandRun {
        exit_code: 0,
        status_success: true,
        launch_error: false,
        duration_ms,
        stdout_digest: "sha256:stdout".to_string(),
        stderr_digest: "sha256:stderr".to_string(),
        failure: Default::default(),
    }
}

fn observe_receipt(
    roundtrip: query_roundtrip::RoundtripQuery,
    status: &str,
    run_id: &str,
    correlation_id: &str,
) -> query_roundtrip::ObserveReceipt {
    query_roundtrip::ObserveReceipt {
        receipt: format!(
            "validation_artifacts/observability/live-loop/commands/{roundtrip:?}.json"
        ),
        exit_code: 0,
        status: status.to_string(),
        value: json!({
            "status": status,
            "roundtrip": format!("{roundtrip:?}"),
            "run_id": run_id,
            "correlation_id": correlation_id
        }),
    }
}

fn write_minimal_manifest(root: &Path) -> String {
    std::fs::create_dir_all(root).expect("root");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    crate::package::inventory::package_digest(root).expect("candidate")
}

#[test]
fn reconciliation_fails_at_each_required_observe_roundtrip() {
    for failing_roundtrip in all_roundtrips() {
        let root = temp_root(&format!("live-loop-roundtrip-{failing_roundtrip:?}"));
        let candidate = write_minimal_manifest(&root);
        let surface = surface_by_id("changed_files").expect("surface");
        let command = measure_command();
        let run = command_run(11);
        let mut roundtrip =
            |roundtrip: query_roundtrip::RoundtripQuery, run_id: &str, correlation_id: &str| {
                if roundtrip == failing_roundtrip {
                    return Err(format!("{failing_roundtrip:?} unavailable"));
                }
                Ok(observe_receipt(roundtrip, "pass", run_id, correlation_id))
            };

        let err = reconcile_with_observe_roundtrip(
            &root,
            surface,
            &candidate,
            &command,
            &run,
            &mut roundtrip,
        )
        .expect_err("roundtrip failure blocks reconciliation");

        assert_eq!(err, format!("{failing_roundtrip:?} unavailable"));
        std::fs::remove_dir_all(root).expect("cleanup failed observe roundtrip");
    }
}

#[test]
fn reconciliation_status_fails_when_any_observe_roundtrip_reports_fail() {
    for failing_roundtrip in all_roundtrips() {
        let root = temp_root(&format!(
            "live-loop-command-observation-{failing_roundtrip:?}-fail-status"
        ));
        let candidate = write_minimal_manifest(&root);
        let surface = surface_by_id("changed_files").expect("surface");
        let command = measure_command();
        let run = command_run(11);
        let mut roundtrip =
            |roundtrip: query_roundtrip::RoundtripQuery, run_id: &str, correlation_id: &str| {
                let status = if roundtrip == failing_roundtrip {
                    "fail"
                } else {
                    "pass"
                };
                Ok(observe_receipt(roundtrip, status, run_id, correlation_id))
            };

        let reconciliation = reconcile_with_observe_roundtrip(
            &root,
            surface,
            &candidate,
            &command,
            &run,
            &mut roundtrip,
        )
        .expect("nonpassing query receipt still produces reconciliation");

        assert_eq!(
            reconciliation.status,
            "query_or_explain_reconciliation_failed"
        );
        let reconciliation = reconciliation.value;
        assert_eq!(
            reconciliation[roundtrip_key(failing_roundtrip)]["status"],
            "fail"
        );
        assert_eq!(
            reconciliation["claim_impact"],
            "source_local_live_loop_node_observation_only_not_speed_claim"
        );
        std::fs::remove_dir_all(root).expect("cleanup failed-status observe roundtrip");
    }
}

#[test]
fn reconciliation_passes_when_all_observe_roundtrips_match_command_run() {
    let root = temp_root("live-loop-command-observation-pass");
    let candidate = write_minimal_manifest(&root);
    let surface = surface_by_id("changed_files").expect("surface");
    let command = measure_command();
    let run = command_run(11);
    let mut observed_roundtrips = Vec::new();
    let mut roundtrip =
        |roundtrip: query_roundtrip::RoundtripQuery, run_id: &str, correlation_id: &str| {
            observed_roundtrips.push((roundtrip, run_id.to_string(), correlation_id.to_string()));
            Ok(observe_receipt(roundtrip, "pass", run_id, correlation_id))
        };

    let reconciliation = reconcile_with_observe_roundtrip(
        &root,
        surface,
        &candidate,
        &command,
        &run,
        &mut roundtrip,
    )
    .expect("successful reconciliation");

    assert_eq!(reconciliation.status, "pass");
    let reconciliation = reconciliation.value;
    assert_eq!(
        reconciliation["claim_impact"],
        "source_local_live_loop_node_observation_only_not_speed_claim"
    );
    let run_id = reconciliation["run_id"].as_str().expect("run id");
    let correlation_id = reconciliation["correlation_id"]
        .as_str()
        .expect("correlation id");
    assert_eq!(observed_roundtrips.len(), all_roundtrips().len());
    assert!(
        observed_roundtrips
            .iter()
            .all(|(_, observed_run, observed_correlation)| {
                observed_run == run_id && observed_correlation == correlation_id
            })
    );
    assert_eq!(reconciliation["logs_query"]["status"], "pass");
    assert_eq!(reconciliation["metrics_query"]["status"], "pass");
    assert_eq!(reconciliation["traces_query"]["status"], "pass");
    assert_eq!(reconciliation["explain_failure"]["status"], "pass");
    std::fs::remove_dir_all(root).expect("cleanup successful command observation");
}

fn all_roundtrips() -> [query_roundtrip::RoundtripQuery; 4] {
    [
        query_roundtrip::RoundtripQuery::Logs,
        query_roundtrip::RoundtripQuery::Metrics,
        query_roundtrip::RoundtripQuery::Traces,
        query_roundtrip::RoundtripQuery::ExplainFailure,
    ]
}

fn roundtrip_key(roundtrip: query_roundtrip::RoundtripQuery) -> &'static str {
    match roundtrip {
        query_roundtrip::RoundtripQuery::Logs => "logs_query",
        query_roundtrip::RoundtripQuery::Metrics => "metrics_query",
        query_roundtrip::RoundtripQuery::Traces => "traces_query",
        query_roundtrip::RoundtripQuery::ExplainFailure => "explain_failure",
    }
}
