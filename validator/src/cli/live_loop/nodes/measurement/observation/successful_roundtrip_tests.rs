use super::receipts::{command_run, measure_command, observe_receipt, write_minimal_manifest};
use super::*;
use crate::cli::live_loop::surfaces::surface_by_id;
use crate::self_tests::boundaries::workspace_fixtures::temp_root;

#[test]
fn reconciliation_passes_when_all_observe_roundtrips_match_command_run() {
    let root = temp_root("live-loop-command-observation-pass");
    let candidate = write_minimal_manifest(&root);
    let surface = surface_by_id("changed_files").expect("surface");
    let command = measure_command();
    let run = command_run(11);
    let mut observed_roundtrips = Vec::new();
    let mut roundtrip = |roundtrip: query::RoundtripQuery, run_id: &str, correlation_id: &str| {
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
    assert_eq!(reconciliation["roundtrip_durations_ms"]["logs_query"], 7);
    assert_eq!(reconciliation["roundtrip_durations_ms"]["metrics_query"], 7);
    assert_eq!(reconciliation["roundtrip_durations_ms"]["traces_query"], 7);
    assert_eq!(
        reconciliation["roundtrip_durations_ms"]["explain_failure"],
        7
    );
    assert_eq!(reconciliation["slowest_roundtrip"]["duration_ms"], 7);
    assert_eq!(
        reconciliation["first_failed_roundtrip"],
        serde_json::Value::Null
    );
    std::fs::remove_dir_all(root).expect("cleanup successful command observation");
}

fn all_roundtrips() -> [query::RoundtripQuery; 4] {
    [
        query::RoundtripQuery::Logs,
        query::RoundtripQuery::Traces,
        query::RoundtripQuery::Metrics,
        query::RoundtripQuery::ExplainFailure,
    ]
}
