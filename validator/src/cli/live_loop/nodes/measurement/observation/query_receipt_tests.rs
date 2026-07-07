use super::*;
use crate::cli::live_loop::surfaces::surface_by_id;

#[test]
fn unavailable_backend_receipt_is_bounded_and_agent_legible() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "live-loop-backend-unavailable-receipt",
    );
    std::fs::create_dir_all(&root).expect("root");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    let surface = surface_by_id("changed_files").expect("surface");
    let receipt = receipt_path(surface.id, "logs-query");
    let backend = backend_for(LiveQueryRoundtrip::Logs);

    let value = write_unavailable_query_receipt(
        &root,
        surface,
        LiveQueryRoundtrip::Logs,
        backend,
        &receipt,
        "run-live-loop",
        "corr-live-loop",
    )
    .expect("unavailable receipt");

    assert_eq!(value["status"], "fail");
    assert_eq!(
        value["failure_class"],
        "live_loop_observability_backend_unavailable"
    );
    assert_eq!(value["query_kind"], "logs");
    assert_eq!(value["rows"], json!([]));
    assert_eq!(value["supported_claims"], json!([]));
    assert_eq!(value["backend_service"], "victorialogs");
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let store = crate::schema_catalog::load(workspace_root);
    let schema_failures = crate::schema_catalog::schema_errors(
        &store,
        "observability-query-result.schema.json",
        &value,
    );
    assert!(schema_failures.is_empty(), "{schema_failures:?}");
    assert_eq!(
        value["candidate_digest"],
        crate::package::inventory::package_digest(&root).expect("candidate")
    );
    assert_eq!(value["row_count"], 0);
    assert_eq!(value["bounded_output_status"], "pass");
    assert_eq!(value["redaction_status"], "pass");
    assert!(
        value["next_repair"]
            .as_str()
            .expect("next repair")
            .contains("observe stack health")
    );
    assert!(
        crate::json_boundary::read_json(&root.join(receipt)).expect("receipt on disk")
            ["result_digest"]
            .as_str()
            .expect("result digest")
            .starts_with("sha256:")
    );
    std::fs::remove_dir_all(root).expect("cleanup backend unavailable receipt");
}

#[test]
fn backend_unavailable_receipt_queries_are_bounded_by_backend_kind() {
    assert_eq!(query_kind(BackendQuery::Logs), "logs");
    assert_eq!(query_kind(BackendQuery::Metrics), "metrics");
    assert_eq!(query_kind(BackendQuery::Traces), "traces");
    assert_eq!(
        query_text(BackendQuery::Logs, "run-log", "corr-log"),
        "_time:5m run_id:run-log correlation_id:corr-log"
    );
    assert_eq!(
        query_text(BackendQuery::Metrics, "run-metric", "corr-metric"),
        crate::cli::observe::query::bounded_metric_query_for_operation(
            crate::cli::observe::types::ObserveOperation::MetricsQuery.id()
        )
    );
    assert_eq!(
        query_text(BackendQuery::Traces, "run-trace", "corr-trace"),
        "{run_id=\"run-trace\", correlation_id=\"corr-trace\"}"
    );
}
