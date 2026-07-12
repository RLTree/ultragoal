use super::*;
use crate::cli::observe::types::{ObserveCommand, ObserveOperation};
use serde_json::json;

fn command(operation: ObserveOperation) -> ObserveCommand {
    ObserveCommand {
        operation,
        receipt: None,
        query: None,
        run_id: None,
        correlation_id: None,
        claim_id: None,
        check_id: None,
        law_id: None,
        target_command: None,
        target_family: None,
        row_limit: 100,
        byte_limit: 4096,
        timeout_ms: 1000,
    }
}

#[test]
fn successor_catalog_unavailable_is_stable_and_does_not_read_static_inventory() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("observe-telemetry-summary");
    let inventory = root.join("docs/generated/observability");
    std::fs::create_dir_all(&inventory).expect("inventory dir");
    let path = inventory.join("command-inventory.json");
    std::fs::write(&path, "SECRET_CANARY").expect("attacker bytes");
    let first = inventory_status::complete(&root).expect_err("catalog unavailable");
    std::fs::write(&path, [0xff, 0xfe]).expect("non-json bytes");
    let second = inventory_status::complete(&root).expect_err("catalog unavailable");
    std::fs::remove_file(&path).expect("remove attacker file");
    let missing = inventory_status::complete(&root).expect_err("catalog unavailable");

    assert_eq!(first, second);
    assert_eq!(second, missing);
    assert!(first.starts_with("HCT-OBSERVE successor catalog unavailable/not adopted"));
    assert!(!first.contains("SECRET_CANARY"));
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn claim_repairs_cover_current_failure_classes() {
    assert_eq!(
        claims::next_repair_for(
            ObserveOperation::Prove,
            "fail",
            Some("first_failure=observability_command_telemetry_query_not_current")
        ),
        "refresh the first same-candidate query proof named in why_failed: rerun the target command, query logs metrics traces for that run, then rerun observe prove"
    );
    assert_eq!(
        claims::next_repair_for(
            ObserveOperation::Prove,
            "fail",
            Some("first_failure=observability_command_telemetry_receipt_missing")
        ),
        "refresh the first command receipt named in why_failed on the current candidate, then query logs metrics traces and rerun observe prove"
    );
    assert_eq!(
        claims::next_repair_for(
            ObserveOperation::ExplainFailure,
            "fail",
            Some("observed telemetry candidate digest mismatch")
        ),
        "rerun target command on the current candidate before claiming observability command telemetry"
    );
    assert_eq!(
        claims::next_repair_for(command(ObserveOperation::LogsQuery).operation, "pass", None),
        "keep receipt same-candidate and rerun source audit before any readiness claim"
    );
    assert_eq!(
        claims::next_repair_for(
            ObserveOperation::TracesQuery,
            "fail",
            Some("observability query returned no matching rows")
        ),
        "trace backend did not return a same-candidate span tree inside the bounded query window; keep the row partial, inspect exporter ingestion latency and trace tag projection, rerun the target command once, then rerun observe traces query by run_id/correlation_id/current digest"
    );
    assert_eq!(
        claims::next_repair_for(
            ObserveOperation::TracesQuery,
            "fail",
            Some("victoriatraces trace lookup returned 404 before the span tree was queryable")
        ),
        "trace backend did not return a same-candidate span tree inside the bounded query window; keep the row partial, inspect exporter ingestion latency and trace tag projection, rerun the target command once, then rerun observe traces query by run_id/correlation_id/current digest"
    );
    assert_eq!(
        claims::next_repair_for(
            ObserveOperation::MetricsQuery,
            "fail",
            Some("observability_metric_event_time_stale:metric_event_unix=1 target_event_unix=2")
        ),
        "metrics backend returned an older sample than the target command event; keep the row partial, inspect metric exporter timestamp/import path and bounded PromQL selector, rerun the target command once, then rerun observe metrics query by run_id/correlation_id/current digest"
    );
    assert_eq!(
        claims::next_repair_for(
            ObserveOperation::MetricsQuery,
            "fail",
            Some("observability query returned no matching rows")
        ),
        "metrics backend did not return a bounded current sample for the target command; keep the row partial, inspect metric ingestion latency and the bounded PromQL selector, rerun the target command once, then rerun observe metrics query by run_id/correlation_id/current digest"
    );
}

#[test]
fn query_failure_receipts_name_specific_backend_failure_class() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "observe-query-specific-failure-class",
    );
    std::fs::create_dir_all(&root).expect("root");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");

    let metrics = base_receipt_for_candidate(
        &root,
        &command(ObserveOperation::MetricsQuery),
        "fail",
        Some("observability query returned no matching rows"),
        candidate.clone(),
    )
    .expect("metrics receipt");
    assert_eq!(
        metrics["failure_class"],
        "observability_metric_missing_for_target"
    );
    assert_eq!(
        metrics["event"]["failure_class"],
        "observability_metric_missing_for_target"
    );
    assert!(
        metrics["next_repair"]
            .as_str()
            .expect("next repair")
            .contains("metric ingestion latency")
    );

    let traces = base_receipt_for_candidate(
        &root,
        &command(ObserveOperation::TracesQuery),
        "fail",
        Some("victoriatraces trace lookup returned 404 before the span tree was queryable"),
        candidate,
    )
    .expect("traces receipt");
    assert_eq!(
        traces["failure_class"],
        "observability_trace_tree_unavailable"
    );
    assert_eq!(
        traces["event"]["failure_class"],
        "observability_trace_tree_unavailable"
    );

    let logs_timeout = base_receipt_for_candidate(
        &root,
        &command(ObserveOperation::LogsQuery),
        "fail",
        Some("curl query failed: operation timed out"),
        crate::package::inventory::package_digest(&root).expect("candidate"),
    )
    .expect("logs timeout receipt");
    assert_eq!(
        logs_timeout["failure_class"],
        "observability_live_backend_timeout"
    );
    let logs_empty = base_receipt_for_candidate(
        &root,
        &command(ObserveOperation::LogsQuery),
        "fail",
        Some("observability query returned no matching rows"),
        crate::package::inventory::package_digest(&root).expect("candidate"),
    )
    .expect("logs empty receipt");
    assert_eq!(
        logs_empty["failure_class"],
        "observability_log_record_unavailable"
    );
    let stale_logs = base_receipt_for_candidate(
        &root,
        &command(ObserveOperation::LogsQuery),
        "fail",
        Some("observability_logs_candidate_mismatch:sha256:old!=sha256:current"),
        crate::package::inventory::package_digest(&root).expect("candidate"),
    )
    .expect("logs candidate mismatch receipt");
    assert_eq!(
        stale_logs["failure_class"],
        "observability_candidate_mismatch"
    );
    let stale_metrics = base_receipt_for_candidate(
        &root,
        &command(ObserveOperation::MetricsQuery),
        "fail",
        Some("observability_metric_candidate_missing"),
        crate::package::inventory::package_digest(&root).expect("candidate"),
    )
    .expect("metrics candidate missing receipt");
    assert_eq!(
        stale_metrics["event"]["failure_class"],
        "observability_candidate_mismatch"
    );
    std::fs::remove_dir_all(root).expect("cleanup query failure class");
}

#[test]
fn telemetry_receipt_text_is_required_for_query_identity() {
    assert_eq!(
        query_receipt_text_for_test(&json!({"run_id":"run-required"}), "run_id").expect("run id"),
        "run-required"
    );
    let error =
        query_receipt_text_for_test(&json!({}), "run_id").expect_err("missing run id fails");
    assert_eq!(error, "observability telemetry receipt missing run_id");
}
