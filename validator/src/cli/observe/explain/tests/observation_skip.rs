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
            "schema": crate::cli::observe::types::EVENT_SCHEMA,
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
            "schema": crate::cli::observe::types::EVENT_SCHEMA,
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
            "schema": crate::cli::observe::types::EVENT_SCHEMA,
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

    let receipt = super::run(&root, &command).expect("explain");

    assert_eq!(receipt["observed_run"]["operation"], "source.audit");
    assert_eq!(receipt["failure_class"], "source_audit_check_failure");
    fs::remove_dir_all(root).expect("cleanup explain skip");
}
