use super::super::observe_receipt_reader::receipt_path;
use super::receipt_capture as subject;
use super::{LiveQueryRoundtrip, PendingQueryReceipt, RoundtripQuery};
use crate::cli::live_loop::surfaces::surface_by_id;
use serde_json::json;

#[test]
fn live_query_receipt_serializes_failed_observation_without_erasing_validation_result() {
    let root = prepared_root("live-query-capture-failed-observation");
    let surface = surface_by_id("changed_files").expect("surface");
    let receipt = receipt_path(surface.id, "logs-query");
    let command = subject::observe_command(
        RoundtripQuery::Logs,
        receipt.clone(),
        "run-live-query",
        "corr-live-query",
    );
    let live = subject::LiveQueryReceipt {
        roundtrip: LiveQueryRoundtrip::Logs,
        receipt: receipt.clone(),
        command,
        observation: crate::cli::observe::query::LiveQueryObservation {
            query: "_time:5m run_id:run-live-query".to_string(),
            output: Err("backend unavailable".to_string()),
            candidate: crate::package::inventory::package_digest(&root).expect("candidate"),
        },
        query_duration_ms: 4,
    };

    let observe_receipt =
        subject::write_pending_query_receipt(&root, PendingQueryReceipt::Live(live))
            .expect("serial failed observation receipt");

    assert_eq!(observe_receipt.exit_code, 1);
    assert_eq!(observe_receipt.status, "fail");
    assert!(observe_receipt.duration_ms >= 4);
    assert_eq!(observe_receipt.value["why_failed"], "backend unavailable");
    assert_eq!(observe_receipt.value["row_count"], 0);
    assert!(root.join(receipt).exists());
    std::fs::remove_dir_all(root).expect("cleanup failed observation");
}

#[test]
fn live_query_receipt_propagates_telemetry_spool_write_failure() {
    let root = prepared_root("live-query-capture-spool-write-failure");
    let spool_path = root.join("validation_artifacts/observability/spool");
    std::fs::create_dir_all(spool_path.parent().expect("spool parent")).expect("spool parent");
    std::fs::write(&spool_path, b"not a directory").expect("spool blocker");
    let surface = surface_by_id("changed_files").expect("surface");
    let receipt = receipt_path(surface.id, "logs-query");
    let command = subject::observe_command(
        RoundtripQuery::Logs,
        receipt.clone(),
        "run-spool-blocked",
        "corr-spool-blocked",
    );
    let live = subject::LiveQueryReceipt {
        roundtrip: LiveQueryRoundtrip::Logs,
        receipt: receipt.clone(),
        command,
        observation: crate::cli::observe::query::LiveQueryObservation {
            query: "_time:5m run_id:run-spool-blocked".to_string(),
            output: Ok("{\"rows\":[]}".to_string()),
            candidate: crate::package::inventory::package_digest(&root).expect("candidate"),
        },
        query_duration_ms: 1,
    };

    let err = subject::write_pending_query_receipt(&root, PendingQueryReceipt::Live(live))
        .expect_err("blocked telemetry spool prevents query receipt claim");

    assert!(err.contains("observability/spool"), "{err}");
    assert!(
        !root.join(receipt).exists(),
        "query receipt is not published when telemetry event capture fails"
    );
    std::fs::remove_dir_all(root).expect("cleanup blocked spool");
}

#[test]
fn live_query_receipt_serializes_same_candidate_query_records() {
    let root = prepared_root("live-query-capture-same-candidate-record");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let surface = surface_by_id("changed_files").expect("surface");
    let receipt = receipt_path(surface.id, "logs-query");
    let command = subject::observe_command(
        RoundtripQuery::Logs,
        receipt.clone(),
        "run-live-query-pass",
        "corr-live-query-pass",
    );
    let event = json!({
        "schema": crate::cli::observe::command::EVENT_SCHEMA,
        "run_id": "run-live-query-pass",
        "correlation_id": "corr-live-query-pass",
        "candidate_digest": candidate,
        "operation": "loop.changed-files",
        "status": "pass",
        "failure_class": "none",
        "why_failed": "none",
        "where_failed": "none",
        "next_repair": "none",
        "claim_impact": "source_local_hot_loop_validation_only",
        "law_id": "law-observability",
        "check_id": "check-live-query-record",
        "claim_id": "claim-live-query-record"
    });
    crate::cli::observe::telemetry::spool_write_for_test(&root, &event).expect("target event");
    let live = subject::LiveQueryReceipt {
        roundtrip: LiveQueryRoundtrip::Logs,
        receipt: receipt.clone(),
        command,
        observation: crate::cli::observe::query::LiveQueryObservation {
            query: "run_id:run-live-query-pass correlation_id:corr-live-query-pass".to_string(),
            output: Ok(event.to_string()),
            candidate: crate::package::inventory::package_digest(&root).expect("current"),
        },
        query_duration_ms: 4,
    };

    let observe_receipt =
        subject::write_pending_query_receipt(&root, PendingQueryReceipt::Live(live))
            .expect("serial same-candidate query receipt");

    assert_eq!(observe_receipt.exit_code, 0);
    assert_eq!(observe_receipt.status, "pass");
    assert_eq!(observe_receipt.value["row_count"], 1);
    assert_eq!(observe_receipt.value["observed_failure_class"], "none");
    assert_eq!(observe_receipt.value["candidate_digest"], candidate);
    assert!(root.join(receipt).exists());
    std::fs::remove_dir_all(root).expect("cleanup same-candidate record");
}

#[test]
fn live_query_receipt_rejects_external_claim_artifact_path_before_claiming_query_proof() {
    let root = prepared_root("live-query-receipt-external-path");
    let receipt = std::path::PathBuf::from("/tmp/live-loop-absolute-logs-query.json");
    let command =
        subject::observe_command(RoundtripQuery::Logs, receipt.clone(), "run-abs", "corr-abs");
    let live = subject::LiveQueryReceipt {
        roundtrip: LiveQueryRoundtrip::Logs,
        receipt: receipt.clone(),
        command,
        observation: crate::cli::observe::query::LiveQueryObservation {
            query: "_time:5m run_id:run-abs".to_string(),
            output: Ok("{\"rows\":[]}".to_string()),
            candidate: crate::package::inventory::package_digest(&root).expect("candidate"),
        },
        query_duration_ms: 1,
    };

    let err = subject::write_pending_query_receipt(&root, PendingQueryReceipt::Live(live))
        .expect_err("absolute receipt path is external debug only");

    assert!(err.contains("external debug only"), "{err}");
    assert!(!receipt.exists());
    std::fs::remove_dir_all(root).expect("cleanup external receipt rejection");
}

#[test]
fn live_query_receipt_reports_query_artifact_publication_failure() {
    let root = prepared_root("live-query-receipt-publication-failure");
    let receipt = receipt_path("changed_files", "logs-query");
    std::fs::create_dir_all(root.join(&receipt)).expect("claim artifact path blocked");
    let command = subject::observe_command(
        RoundtripQuery::Logs,
        receipt.clone(),
        "run-missing-manifest",
        "corr-missing-manifest",
    );
    let live = subject::LiveQueryReceipt {
        roundtrip: LiveQueryRoundtrip::Logs,
        receipt,
        command,
        observation: crate::cli::observe::query::LiveQueryObservation {
            query: "_time:5m run_id:run-missing-manifest".to_string(),
            output: Ok("{\"rows\":[]}".to_string()),
            candidate: "sha256:test-candidate".to_string(),
        },
        query_duration_ms: 1,
    };

    let err = subject::write_pending_query_receipt(&root, PendingQueryReceipt::Live(live))
        .expect_err("directory at receipt path blocks claim artifact publication");

    assert!(
        err.contains("rename failed") || err.contains("json"),
        "{err}"
    );
    std::fs::remove_dir_all(root).expect("cleanup publication failure");
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
