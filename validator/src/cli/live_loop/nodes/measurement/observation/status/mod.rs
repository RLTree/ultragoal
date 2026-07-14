use super::receipts::{command_run, measure_command, observe_receipt, write_minimal_manifest};
use super::*;
use crate::cli::live_loop::surfaces::surface_by_id;
use crate::self_tests::boundaries::workspace_fixtures::temp_root;
use serde_json::json;

#[path = "../roundtrip_failure_tests.rs"]
mod roundtrip_failure;
#[path = "../successful_roundtrip_tests.rs"]
mod success;

#[test]
fn reconciliation_propagates_command_observation_write_failure() {
    let root = temp_root("live-loop-command-observation-helper-write-failure");
    let candidate = write_minimal_manifest(&root);
    let surface = surface_by_id("changed_files").expect("surface");
    std::fs::create_dir_all(root.join(super::event::receipt_path(surface.id)))
        .expect("claim artifact path blocked");
    let command = measure_command();
    let run = command_run(11);
    let mut roundtrip = |roundtrip: query::RoundtripQuery, run_id: &str, correlation_id: &str| {
        Ok(observe_receipt(roundtrip, "pass", run_id, correlation_id))
    };
    let callback_probe =
        roundtrip(query::RoundtripQuery::Logs, "run-probe", "corr-probe").expect("probe callback");
    assert_eq!(callback_probe.status, "pass");
    assert_eq!(callback_probe.value["run_id"], "run-probe");
    assert_eq!(callback_probe.value["correlation_id"], "corr-probe");

    let err = reconcile_with_observe_roundtrip(
        &root,
        surface,
        &candidate,
        &command,
        &run,
        &mut roundtrip,
    )
    .expect_err("command observation write failure propagates");

    assert_publication_failure(&err);
    std::fs::remove_dir_all(root).expect("cleanup helper write failure");
}

#[test]
fn reconciliation_reports_query_receipt_publication_failure() {
    let root = temp_root("live-loop-query-receipt-publication-failure");
    let candidate = write_minimal_manifest(&root);
    let surface = surface_by_id("changed_files").expect("surface");
    std::fs::create_dir_all(root.join(query::receipt_path(surface.id, "logs-query")))
        .expect("logs query receipt path blocked");
    let command = measure_command();
    let run = command_run(11);

    let reconciliation = reconcile(&root, surface, &candidate, &command, &run);

    assert_eq!(reconciliation.status, "command_observation_failed");
    assert_publication_failure(reconciliation.value["failure"].as_str().unwrap_or_default());
    assert_eq!(
        reconciliation.value["claim_impact"],
        "live_loop_node_timing_blocked"
    );
    std::fs::remove_dir_all(root).expect("cleanup query publication failure");
}

#[test]
fn reconciliation_reports_later_query_receipt_publication_failures() {
    for suffix in ["metrics-query", "traces-query"] {
        let root = temp_root(&format!("live-loop-{suffix}-receipt-publication-failure"));
        let candidate = write_minimal_manifest(&root);
        let surface = surface_by_id("changed_files").expect("surface");
        std::fs::create_dir_all(root.join(query::receipt_path(surface.id, suffix)))
            .expect("query receipt path blocked");
        let command = measure_command();
        let run = command_run(11);

        let reconciliation = reconcile(&root, surface, &candidate, &command, &run);

        assert_eq!(reconciliation.status, "command_observation_failed");
        assert_publication_failure(reconciliation.value["failure"].as_str().unwrap_or_default());
        assert_eq!(
            reconciliation.value["claim_impact"],
            "live_loop_node_timing_blocked"
        );
        std::fs::remove_dir_all(root).expect("cleanup later query publication failure");
    }
}

#[test]
fn query_capture_worker_panic_is_reported_as_typed_join_failure() {
    std::thread::scope(|scope| {
        let handle =
            scope.spawn(|| -> query::PendingQueryReceipt { panic!("query capture worker panic") });

        let err = join_query_capture("logs-query", handle)
            .err()
            .expect("worker panic must become typed failure");

        assert_eq!(err, "observe logs-query worker panicked");
    });
}

#[test]
fn reconciliation_reports_explain_receipt_publication_failure() {
    let root = temp_root("live-loop-explain-receipt-publication-failure");
    let candidate = write_minimal_manifest(&root);
    let surface = surface_by_id("changed_files").expect("surface");
    std::fs::create_dir_all(root.join(query::receipt_path(surface.id, "explain-failure")))
        .expect("explain receipt path blocked");
    let command = measure_command();
    let run = command_run(11);

    let reconciliation = reconcile(&root, surface, &candidate, &command, &run);

    assert_eq!(reconciliation.status, "command_observation_failed");
    assert_publication_failure(reconciliation.value["failure"].as_str().unwrap_or_default());
    assert_eq!(
        reconciliation.value["claim_impact"],
        "live_loop_node_timing_blocked"
    );
    std::fs::remove_dir_all(root).expect("cleanup explain publication failure");
}

#[test]
fn reconciliation_rejects_explain_receipt_that_lacks_query_evidence() {
    let root = temp_root("live-loop-command-observation-explain-without-query-evidence");
    let candidate = write_minimal_manifest(&root);
    let surface = surface_by_id("changed_files").expect("surface");
    let command = measure_command();
    let run = command_run(11);
    let mut roundtrip = |roundtrip: query::RoundtripQuery, run_id: &str, correlation_id: &str| {
        let mut receipt = observe_receipt(roundtrip, "pass", run_id, correlation_id);
        if roundtrip == query::RoundtripQuery::ExplainFailure {
            receipt.value["explanation"]["query_evidence"] = json!({
                "logs": {"status":"missing"},
                "metrics": {"status":"missing"},
                "traces": {"status":"missing"}
            });
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
    .expect("missing explain query evidence still reports reconciliation");

    assert_eq!(
        reconciliation.status,
        "query_or_explain_reconciliation_failed"
    );
    assert_eq!(
        reconciliation.value["first_failed_roundtrip"]["roundtrip"],
        "explain_failure"
    );
    std::fs::remove_dir_all(root).expect("cleanup explain query evidence");
}

#[test]
fn reconciliation_rejects_explain_receipt_without_explanation_tree() {
    let root = temp_root("live-loop-command-observation-explain-without-tree");
    let candidate = write_minimal_manifest(&root);
    let surface = surface_by_id("changed_files").expect("surface");
    let command = measure_command();
    let run = command_run(11);
    let mut roundtrip = |roundtrip: query::RoundtripQuery, run_id: &str, correlation_id: &str| {
        let mut receipt = observe_receipt(roundtrip, "pass", run_id, correlation_id);
        if roundtrip == query::RoundtripQuery::ExplainFailure {
            receipt.value["explanation"] = serde_json::Value::Null;
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
    .expect("missing explain tree still reports reconciliation");

    assert_eq!(
        reconciliation.status,
        "query_or_explain_reconciliation_failed"
    );
    assert_eq!(
        reconciliation.value["first_failed_roundtrip"]["roundtrip"],
        "explain_failure"
    );
    std::fs::remove_dir_all(root).expect("cleanup missing explain tree");
}

fn assert_publication_failure(failure: &str) {
    assert!(
        ["rename failed", "json"]
            .into_iter()
            .any(|needle| failure.contains(needle)),
        "{failure}"
    );
}
