use super::super::observe_receipt_reader::receipt_path;
use super::*;
use crate::cli::live_loop::surfaces::surface_by_id;
use serde_json::json;

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
fn roundtrip_query_metadata_covers_every_product_operation() {
    assert_eq!(RoundtripQuery::Logs.operation().id(), "observe.logs.query");
    assert_eq!(
        RoundtripQuery::Metrics.operation().id(),
        "observe.metrics.query"
    );
    assert_eq!(
        RoundtripQuery::Traces.operation().id(),
        "observe.traces.query"
    );
    assert_eq!(
        RoundtripQuery::ExplainFailure.operation().id(),
        "observe.explain-failure"
    );
    assert_eq!(RoundtripQuery::Logs.receipt_suffix(), "logs-query");
    assert_eq!(RoundtripQuery::Metrics.receipt_suffix(), "metrics-query");
    assert_eq!(RoundtripQuery::Traces.receipt_suffix(), "traces-query");
    assert_eq!(
        RoundtripQuery::ExplainFailure.receipt_suffix(),
        "explain-failure"
    );
    assert_eq!(LiveQueryRoundtrip::Logs.receipt_suffix(), "logs-query");
    assert_eq!(
        LiveQueryRoundtrip::Metrics.receipt_suffix(),
        "metrics-query"
    );
    assert_eq!(LiveQueryRoundtrip::Traces.receipt_suffix(), "traces-query");
    assert_eq!(
        LiveQueryRoundtrip::Logs.query_kind(),
        crate::cli::observe::query::QueryKind::Logs
    );
}

#[test]
fn ready_backend_capture_defers_live_query_receipt_until_serial_write() {
    let root = prepared_root("live-loop-ready-backend-query-capture");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let surface = surface_by_id("changed_files").expect("surface");
    let receipt = receipt_path(surface.id, "logs-query");

    let pending = capture_query_roundtrip(
        &root,
        surface,
        LiveQueryRoundtrip::Logs,
        "run-ready-capture",
        "corr-ready-capture",
        &candidate,
    );

    assert!(
        !root.join(&receipt).exists(),
        "parallel capture step must not publish shared validation_artifacts receipts"
    );
    let receipt = write_pending_query_receipt(&root, pending).expect("serial query receipt");

    assert!(receipt.duration_ms >= 1);
    assert!(
        ["pass", "fail"].contains(&receipt.status.as_str()),
        "backend availability changes status, not the capture/write contract"
    );
    assert!(root.join(receipt_path(surface.id, "logs-query")).exists());
    std::fs::remove_dir_all(root).expect("cleanup ready backend capture");
}
