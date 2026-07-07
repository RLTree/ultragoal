use serde_json::json;
use std::fs;

#[test]
fn explain_does_not_keep_stale_observation_failure_after_later_query_pass() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "observe-explain-stale-observation-failure",
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
            "schema": crate::cli::observe::types::EVENT_SCHEMA,
            "run_id": "run-observation-later-pass",
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
    .expect("product pass event");
    crate::cli::observe::telemetry::spool_write_for_test(
        &root,
        &json!({
            "schema": crate::cli::observe::types::EVENT_SCHEMA,
            "run_id": "run-observation-later-pass",
            "candidate_digest": candidate,
            "operation": "observe.traces.query",
            "status": "fail",
            "failure_class": "observability_trace_tree_unavailable",
            "why_failed": "trace backend had not ingested the span yet",
            "where_failed": "observe.traces.query.live_backend",
            "next_repair": "rerun observe traces query",
            "claim_impact": "observability_reconciliation_blocked"
        }),
    )
    .expect("failed observation event");
    crate::cli::observe::telemetry::spool_write_for_test(
        &root,
        &json!({
            "schema": crate::cli::observe::types::EVENT_SCHEMA,
            "run_id": "run-observation-later-pass",
            "candidate_digest": candidate,
            "operation": "observe.traces.query",
            "status": "pass",
            "failure_class": "none",
            "why_failed": "none",
            "where_failed": "none",
            "next_repair": "none",
            "claim_impact": "observability_evidence_only"
        }),
    )
    .expect("later observation pass event");
    let command = crate::cli::observe::parse(&[
        "observe".to_string(),
        "explain-failure".to_string(),
        "--run-id".to_string(),
        "run-observation-later-pass".to_string(),
    ])
    .expect("parse")
    .expect("observe");

    let receipt = super::super::run(&root, &command).expect("explain");

    assert_eq!(receipt["observed_run"]["operation"], "source.audit");
    assert_eq!(receipt["failure_class"], "none");
    assert_eq!(receipt["status"], "pass");
    fs::remove_dir_all(root).expect("cleanup stale observation failure");
}
