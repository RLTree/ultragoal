use crate::cli::observe::query::QueryKind;
use serde_json::json;

#[test]
fn live_result_failure_accepts_same_candidate_log_rows_after_record_reconciliation() {
    let root = super::prepare_root("query-live-result-logs-reconcile");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let event = json!({
        "schema": crate::cli::observe::types::EVENT_SCHEMA,
        "run_id": "run-query-bound",
        "candidate_digest": candidate,
        "operation": "source.audit",
        "status": "pass",
        "failure_class": "none",
        "why_failed": "none"
    });
    crate::cli::observe::telemetry::spool_write_for_test(&root, &event).expect("target event");

    assert_eq!(
        super::super::live_result_failure(
            &root,
            &super::command(),
            QueryKind::Logs,
            &event.to_string(),
            &candidate
        ),
        None
    );
    std::fs::remove_dir_all(root).expect("cleanup live logs result");
}
