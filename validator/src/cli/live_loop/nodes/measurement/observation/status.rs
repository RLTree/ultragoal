use super::receipts::{command_run, measure_command, observe_receipt, write_minimal_manifest};
use super::*;
use crate::cli::live_loop::surfaces::surface_by_id;
use crate::self_tests::boundaries::workspace_fixtures::temp_root;
use serde_json::json;

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
fn reconciliation_status_rejects_pass_shaped_query_without_bounds_proof() {
    let root = temp_root("live-loop-command-observation-pass-shaped-query");
    let candidate = write_minimal_manifest(&root);
    let surface = surface_by_id("changed_files").expect("surface");
    let command = measure_command();
    let run = command_run(11);
    let mut roundtrip =
        |roundtrip: query_roundtrip::RoundtripQuery, run_id: &str, correlation_id: &str| {
            let mut receipt = observe_receipt(roundtrip, "pass", run_id, correlation_id);
            if roundtrip == query_roundtrip::RoundtripQuery::Metrics {
                receipt.value["bounded_output_status"] = json!("fail");
            }
            Ok(receipt)
        };

    let reconciliation = reconcile_with_observe_roundtrip(
        &root,
        surface,
        &candidate,
        &command,
        &run,
        &mut roundtrip,
    )
    .expect("pass-shaped query still produces reconciliation report");

    assert_eq!(
        reconciliation.status,
        "query_or_explain_reconciliation_failed"
    );
    assert_eq!(reconciliation.value["metrics_query"]["status"], "pass");
    assert_eq!(
        reconciliation.value["metrics_query"]["value"]["bounded_output_status"],
        "fail"
    );
    std::fs::remove_dir_all(root).expect("cleanup pass-shaped query");
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
        query_roundtrip::RoundtripQuery::Traces,
        query_roundtrip::RoundtripQuery::Metrics,
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
