use serde_json::json;
use std::fs;

#[test]
fn explain_skips_observe_explain_events_when_selecting_target_run() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("observe-explain-skip");
    fs::create_dir_all(&root).expect("root");
    fs::write(root.join("owned.txt"), "owned").expect("owned");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["owned.txt"]}),
    )
    .expect("manifest");
    let candidate = crate::package::inventory::package_digest(&root).expect("digest");
    crate::cli::observe::telemetry::spool_write_for_test(
        &root,
        &json!({
            "schema": crate::cli::observe::command::EVENT_SCHEMA,
            "run_id": "run-observed",
            "candidate_digest": candidate,
            "operation": "source.audit",
            "status": "fail",
            "failure_class": "source_audit_check_failure",
            "why_failed": "coverage receipt stale",
            "where_failed": "source_audit.coverage",
            "next_repair": "rerun exact coverage",
            "claim_impact": "readiness_release_completion_update_goal_blocked"
        }),
    )
    .expect("target event");
    crate::cli::observe::telemetry::spool_write_for_test(
        &root,
        &json!({
            "schema": crate::cli::observe::command::EVENT_SCHEMA,
            "run_id": "run-observed",
            "candidate_digest": candidate,
            "operation": "observe.explain-failure",
            "status": "pass",
            "failure_class": "none"
        }),
    )
    .expect("observation event");
    crate::cli::observe::telemetry::spool_write_for_test(
        &root,
        &json!({
            "schema": crate::cli::observe::command::EVENT_SCHEMA,
            "run_id": "run-observed",
            "candidate_digest": candidate,
            "operation": "observe.logs.query",
            "status": "pass",
            "failure_class": "none"
        }),
    )
    .expect("query observation event");
    let command = crate::cli::observe::parse(&[
        "observe".to_string(),
        "explain-failure".to_string(),
        "--run-id".to_string(),
        "run-observed".to_string(),
    ])
    .expect("parse")
    .expect("observe");

    let receipt = super::super::run(&root, &command).expect("explain");

    assert_eq!(receipt["observed_run"]["operation"], "source.audit");
    assert_eq!(receipt["failure_class"], "source_audit_check_failure");
    fs::remove_dir_all(root).expect("cleanup explain skip");
}

#[test]
fn explain_targets_failed_observation_when_product_event_passed() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "observe-explain-observation-failure",
    );
    fs::create_dir_all(&root).expect("root");
    fs::write(root.join("owned.txt"), "owned").expect("owned");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["owned.txt"]}),
    )
    .expect("manifest");
    let candidate = crate::package::inventory::package_digest(&root).expect("digest");
    crate::cli::observe::telemetry::spool_write_for_test(
        &root,
        &json!({
            "schema": crate::cli::observe::command::EVENT_SCHEMA,
            "run_id": "run-observed-query-failure",
            "candidate_digest": candidate,
            "operation": "source.audit",
            "status": "pass",
            "failure_class": "none",
            "why_failed": "none",
            "where_failed": "none",
            "next_repair": "none",
            "claim_impact": "observability_evidence_only"
        }),
    )
    .expect("target event");
    crate::cli::observe::telemetry::spool_write_for_test(
        &root,
        &json!({
            "schema": crate::cli::observe::command::EVENT_SCHEMA,
            "run_id": "run-observed-query-failure",
            "candidate_digest": candidate,
            "operation": "observe.traces.query",
            "status": "fail",
            "failure_class": "observability_trace_tree_unavailable",
            "why_failed": "victoriatraces trace lookup returned 404 before the span tree was queryable",
            "where_failed": "observe.traces.query.live_backend",
            "next_repair": "keep the row partial, inspect exporter ingestion latency and rerun observe traces query",
            "claim_impact": "observability_reconciliation_blocked",
            "command": "ultragoal",
            "subcommand": "observe traces query"
        }),
    )
    .expect("failed observation event");
    let command = crate::cli::observe::parse(&[
        "observe".to_string(),
        "explain-failure".to_string(),
        "--run-id".to_string(),
        "run-observed-query-failure".to_string(),
    ])
    .expect("parse")
    .expect("observe");

    let receipt = super::super::run(&root, &command).expect("explain");

    assert_eq!(receipt["observed_run"]["operation"], "observe.traces.query");
    assert_eq!(receipt["observed_status"], "fail");
    assert_eq!(
        receipt["failure_class"],
        "observability_trace_tree_unavailable"
    );
    assert!(
        receipt["explanation"]["root_cause"]
            .as_str()
            .expect("root cause")
            .contains("observability_trace_tree_unavailable")
    );
    fs::remove_dir_all(root).expect("cleanup observation failure");
}

#[test]
fn explain_keeps_failed_product_event_ahead_of_failed_observation_event() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "observe-explain-product-failure-first",
    );
    fs::create_dir_all(&root).expect("root");
    fs::write(root.join("owned.txt"), "owned").expect("owned");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["owned.txt"]}),
    )
    .expect("manifest");
    let candidate = crate::package::inventory::package_digest(&root).expect("digest");
    crate::cli::observe::telemetry::spool_write_for_test(
        &root,
        &json!({
            "schema": crate::cli::observe::command::EVENT_SCHEMA,
            "run_id": "run-product-failure-first",
            "candidate_digest": candidate,
            "operation": "source.audit",
            "status": "fail",
            "failure_class": "source_audit_check_failure",
            "why_failed": "coverage receipt stale",
            "where_failed": "source_audit.coverage",
            "next_repair": "rerun exact coverage",
            "claim_impact": "readiness_release_completion_update_goal_blocked"
        }),
    )
    .expect("target event");
    crate::cli::observe::telemetry::spool_write_for_test(
        &root,
        &json!({
            "schema": crate::cli::observe::command::EVENT_SCHEMA,
            "run_id": "run-product-failure-first",
            "candidate_digest": candidate,
            "operation": "observe.metrics.query",
            "status": "fail",
            "failure_class": "observability_metric_event_time_stale",
            "why_failed": "metrics backend returned an older sample",
            "where_failed": "observe.metrics.query.live_backend",
            "next_repair": "rerun observe metrics query",
            "claim_impact": "observability_reconciliation_blocked"
        }),
    )
    .expect("failed observation event");
    let command = crate::cli::observe::parse(&[
        "observe".to_string(),
        "explain-failure".to_string(),
        "--run-id".to_string(),
        "run-product-failure-first".to_string(),
    ])
    .expect("parse")
    .expect("observe");

    let receipt = super::super::run(&root, &command).expect("explain");

    assert_eq!(receipt["observed_run"]["operation"], "source.audit");
    assert_eq!(receipt["failure_class"], "source_audit_check_failure");
    fs::remove_dir_all(root).expect("cleanup product failure first");
}
