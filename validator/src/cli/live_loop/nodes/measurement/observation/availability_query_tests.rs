use super::*;
use crate::cli::live_loop::surfaces::surface_by_id;
use serde_json::{Value, json};

#[test]
fn roundtrip_timeouts_keep_live_queries_bounded() {
    assert_eq!(
        RoundtripQuery::ExplainFailure.timeout_ms(),
        EXPLAIN_TIMEOUT_MS
    );
    assert_eq!(
        LiveQueryRoundtrip::Logs.timeout_ms(),
        LIVE_BACKEND_QUERY_TIMEOUT_MS
    );
    assert_eq!(
        LiveQueryRoundtrip::Metrics.timeout_ms(),
        LIVE_BACKEND_QUERY_TIMEOUT_MS
    );
    assert_eq!(
        LiveQueryRoundtrip::Traces.timeout_ms(),
        TRACE_QUERY_TIMEOUT_MS
    );
    assert!(LiveQueryRoundtrip::Traces.timeout_ms() > LIVE_BACKEND_QUERY_TIMEOUT_MS);
    assert_eq!(LIVE_BACKEND_QUERY_TIMEOUT_MS, 3_000);
    assert!(RoundtripQuery::ExplainFailure.timeout_ms() < LIVE_BACKEND_QUERY_TIMEOUT_MS);
}

#[test]
fn unavailable_backend_state_returns_agent_legible_receipt() {
    let root = prepared_root("live-loop-query-roundtrip-unavailable-backend");
    let surface = surface_by_id("changed_files").expect("surface");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let receipt_path = receipt_path(surface.id, "logs-query");
    let backend = super::super::super::live_backend::backend_for(LiveQueryRoundtrip::Logs);
    let receipt = run_with_backend_state(
        &root,
        surface,
        LiveQueryRoundtrip::Logs,
        receipt_path,
        "run-unavailable",
        "corr-unavailable",
        BackendState::Unavailable(backend),
        &candidate,
    )
    .expect("unavailable backend receipt");

    assert_eq!(receipt.exit_code, 1);
    assert_eq!(receipt.status, "fail");
    assert!(receipt.duration_ms >= 1);
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
fn ready_backend_state_runs_live_query_then_serializes_receipt() {
    let root = prepared_root("live-loop-query-roundtrip-ready-backend");
    let surface = surface_by_id("changed_files").expect("surface");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let receipt_path = receipt_path(surface.id, "logs-query");

    let receipt = run_with_backend_state(
        &root,
        surface,
        LiveQueryRoundtrip::Logs,
        receipt_path.clone(),
        "run-ready-live",
        "corr-ready-live",
        BackendState::Ready,
        &candidate,
    )
    .expect("ready backend roundtrip writes receipt");

    assert!(receipt.duration_ms >= 1);
    assert!(
        ["pass", "fail"].contains(&receipt.status.as_str()),
        "backend query result changes observability status, not validation retention"
    );
    assert_eq!(receipt.value["candidate_digest"], candidate);
    assert!(root.join(receipt_path).exists());
    std::fs::remove_dir_all(root).expect("cleanup ready backend");
}

#[test]
fn query_run_routes_every_live_query_through_backend_state() {
    for (roundtrip, suffix) in [
        (RoundtripQuery::Logs, "logs-query"),
        (RoundtripQuery::Metrics, "metrics-query"),
        (RoundtripQuery::Traces, "traces-query"),
    ] {
        let root = prepared_root(&format!("live-loop-query-run-{suffix}"));
        let surface = surface_by_id("changed_files").expect("surface");
        let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
        let receipt = run(
            &root,
            surface,
            roundtrip,
            "run-live-query-route",
            "corr-live-query-route",
            &candidate,
        )
        .expect("live query run produces bounded receipt");

        assert!(receipt.duration_ms >= 1);
        assert!(
            ["pass", "fail"].contains(&receipt.status.as_str()),
            "backend state changes observability status, not routing"
        );
        assert_eq!(receipt.value["candidate_digest"], candidate);
        assert!(root.join(receipt_path(surface.id, suffix)).exists());
        std::fs::remove_dir_all(root).expect("cleanup live query route");
    }
}

#[test]
fn unavailable_backend_state_propagates_receipt_write_failure() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "live-loop-query-roundtrip-unavailable-write-failure",
    );
    std::fs::create_dir_all(&root).expect("root");
    let surface = surface_by_id("changed_files").expect("surface");
    let candidate = "sha256:missing-package-candidate";
    let receipt_path = receipt_path(surface.id, "logs-query");
    let backend = super::super::super::live_backend::backend_for(LiveQueryRoundtrip::Logs);
    let err = run_with_backend_state(
        &root,
        surface,
        LiveQueryRoundtrip::Logs,
        receipt_path,
        "run-unavailable",
        "corr-unavailable",
        BackendState::Unavailable(backend),
        candidate,
    )
    .expect_err("missing manifest blocks unavailable receipt");

    assert!(err.contains("plugin-manifest-draft.json"), "{err}");
    std::fs::remove_dir_all(root).expect("cleanup unavailable write failure");
}

#[test]
fn live_query_capture_defers_unavailable_receipt_write_until_serial_publication() {
    let root = prepared_root("live-loop-query-roundtrip-deferred-unavailable");
    let surface = surface_by_id("changed_files").expect("surface");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let receipt_path = receipt_path(surface.id, "logs-query");
    let backend = super::super::super::live_backend::backend_for(LiveQueryRoundtrip::Logs);

    let pending = capture_live_query(
        &root,
        surface,
        LiveQueryRoundtrip::Logs,
        receipt_path.clone(),
        "run-deferred",
        "corr-deferred",
        BackendState::Unavailable(backend),
        &candidate,
    );
    assert!(
        !root.join(&receipt_path).exists(),
        "capture step must not write shared validation_artifacts receipts"
    );

    let receipt = write_pending_query_receipt(&root, pending).expect("serial receipt write");

    assert_eq!(receipt.exit_code, 1);
    assert_eq!(receipt.status, "fail");
    assert!(
        root.join(&receipt_path).exists(),
        "serial publication step writes the governed receipt"
    );
    std::fs::remove_dir_all(root).expect("cleanup deferred unavailable receipt");
}

pub(super) fn prepared_root(label: &str) -> std::path::PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    std::fs::create_dir_all(&root).expect("root");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    root
}
