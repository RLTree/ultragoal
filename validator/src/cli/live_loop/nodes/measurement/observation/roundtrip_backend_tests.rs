use super::*;
use crate::cli::live_loop::surfaces::surface_by_id;

#[test]
fn roundtrip_timeouts_keep_live_queries_bounded() {
    assert_eq!(RoundtripQuery::Logs.timeout_ms(), QUERY_TIMEOUT_MS);
    assert_eq!(
        RoundtripQuery::ExplainFailure.timeout_ms(),
        QUERY_TIMEOUT_MS
    );
    assert_eq!(
        RoundtripQuery::Metrics.timeout_ms(),
        METRIC_QUERY_TIMEOUT_MS
    );
    assert_eq!(RoundtripQuery::Traces.timeout_ms(), TRACE_QUERY_TIMEOUT_MS);
}

#[test]
fn unavailable_backend_state_returns_agent_legible_receipt() {
    let root = prepared_root("live-loop-query-roundtrip-unavailable-backend");
    let surface = surface_by_id("changed_files").expect("surface");
    let receipt_path = receipt_path(surface.id, "logs-query");
    let backend =
        super::super::backend_readiness::backend_for(RoundtripQuery::Logs).expect("logs backend");
    let receipt = run_with_backend_state(
        &root,
        surface,
        RoundtripQuery::Logs,
        receipt_path,
        "run-unavailable",
        "corr-unavailable",
        BackendState::Unavailable(backend),
    )
    .expect("unavailable backend receipt");

    assert_eq!(receipt.exit_code, 1);
    assert_eq!(receipt.status, "fail");
    assert_eq!(
        receipt.value["failure_class"],
        "live_loop_observability_backend_unavailable"
    );
    assert!(
        receipt
            .value()
            .get("receipt")
            .and_then(Value::as_str)
            .is_some_and(|path| path.contains("logs-query"))
    );
    std::fs::remove_dir_all(root).expect("cleanup unavailable roundtrip");
}

#[test]
fn unavailable_backend_state_propagates_receipt_write_failure() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "live-loop-query-roundtrip-unavailable-write-failure",
    );
    std::fs::create_dir_all(&root).expect("root");
    let surface = surface_by_id("changed_files").expect("surface");
    let receipt_path = receipt_path(surface.id, "logs-query");
    let backend =
        super::super::backend_readiness::backend_for(RoundtripQuery::Logs).expect("logs backend");
    let err = run_with_backend_state(
        &root,
        surface,
        RoundtripQuery::Logs,
        receipt_path,
        "run-unavailable",
        "corr-unavailable",
        BackendState::Unavailable(backend),
    )
    .expect_err("missing manifest blocks unavailable receipt");

    assert!(err.contains("plugin-manifest-draft.json"), "{err}");
    std::fs::remove_dir_all(root).expect("cleanup unavailable write failure");
}

#[test]
fn ready_or_unrequired_backend_state_does_not_emit_unavailable_receipt() {
    let root = prepared_root("live-loop-query-roundtrip-ready-or-unrequired");
    let surface = surface_by_id("changed_files").expect("surface");
    let receipt_path = receipt_path(surface.id, "logs-query");

    for state in [BackendState::Ready, BackendState::NotRequired] {
        let receipt = unavailable_backend_receipt(
            &root,
            surface,
            RoundtripQuery::Logs,
            &receipt_path,
            "run-ready",
            "corr-ready",
            state,
        )
        .expect("backend state");
        assert!(receipt.is_none());
    }
    std::fs::remove_dir_all(root).expect("cleanup ready state");
}

#[test]
fn backend_probe_result_maps_to_concrete_backend_state() {
    let backend =
        super::super::backend_readiness::backend_for(RoundtripQuery::Logs).expect("logs backend");

    assert!(matches!(
        backend_state_from_probe(backend, true),
        BackendState::Ready
    ));
    assert!(matches!(
        backend_state_from_probe(backend, false),
        BackendState::Unavailable(_)
    ));
}

#[test]
fn malformed_observe_receipt_without_status_fails_with_typed_error() {
    let error = receipt_status(&json!({"schema":"observe-query"}), RoundtripQuery::Logs)
        .expect_err("missing receipt status");

    assert_eq!(error, "observe logs-query receipt missing status");
}

#[test]
fn observe_query_runner_propagates_command_failure() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "live-loop-query-roundtrip-observe-command-failure",
    );
    std::fs::create_dir_all(&root).expect("root");
    let receipt = receipt_path("changed_files", "logs-query");

    let err = run_observe_query(
        &root,
        RoundtripQuery::Logs,
        receipt,
        "run-failure",
        "corr-failure",
    )
    .expect_err("observe command failure");

    assert!(
        err.contains("plugin-manifest-draft.json") || err.contains("package digest"),
        "{err}"
    );
    std::fs::remove_dir_all(root).expect("cleanup observe command failure");
}

#[test]
fn observe_receipt_reader_reports_missing_and_malformed_receipts() {
    let root = prepared_root("live-loop-query-roundtrip-receipt-reader");
    let receipt = receipt_path("changed_files", "logs-query");

    let missing = read_observe_receipt(&root, &receipt, RoundtripQuery::Logs, 0)
        .expect_err("missing receipt");
    assert!(missing.contains("logs-query"), "{missing}");

    let absolute = root.join(&receipt);
    std::fs::create_dir_all(absolute.parent().expect("receipt parent")).expect("receipt parent");
    crate::json_boundary::write_json(&absolute, &json!({"schema":"observe-query"}))
        .expect("malformed receipt");
    let malformed = read_observe_receipt(&root, &receipt, RoundtripQuery::Logs, 0)
        .expect_err("malformed receipt");
    assert_eq!(malformed, "observe logs-query receipt missing status");

    std::fs::remove_dir_all(root).expect("cleanup receipt reader");
}

#[test]
fn observe_receipt_reader_rejects_external_claim_artifact_paths() {
    let root = prepared_root("live-loop-query-roundtrip-external-receipt");
    let err = read_observe_receipt(
        &root,
        std::path::Path::new("/tmp/live-loop-logs-query.json"),
        RoundtripQuery::Logs,
        0,
    )
    .expect_err("external receipt rejected");

    assert!(err.contains("external debug only"), "{err}");
    std::fs::remove_dir_all(root).expect("cleanup external receipt");
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
