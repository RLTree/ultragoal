use crate::cli::observe;

const METRIC_NAMES: &str = "ultragoal_command_total|ultragoal_command_duration_ms|ultragoal_command_task_count|ultragoal_command_queue_depth|ultragoal_command_event_unix_seconds";
const METRIC_GROUPING: &str = "sum by (__name__,command,operation,status,law_id,check_id,claim_id,surface,failure_class,exporter,saturation_status)";

#[test]
fn observe_metric_query_text_covers_all_authority_selectors() {
    for (flag, value, expected) in [
        (
            "--run-id",
            "run-abc",
            format!("{METRIC_GROUPING} (last_over_time({{__name__=~\"{METRIC_NAMES}\""),
        ),
        (
            "--law-id",
            "law-abc",
            format!(
                r#"{METRIC_GROUPING} (last_over_time({{__name__=~"{METRIC_NAMES}",law_id="law-abc""#
            ),
        ),
        (
            "--check-id",
            "check-abc",
            format!(
                r#"{METRIC_GROUPING} (last_over_time({{__name__=~"{METRIC_NAMES}",check_id="check-abc""#
            ),
        ),
        (
            "--claim-id",
            "claim-abc",
            format!(
                r#"{METRIC_GROUPING} (last_over_time({{__name__=~"{METRIC_NAMES}",claim_id="claim-abc""#
            ),
        ),
    ] {
        let query = observe::query::query_text(&super::command(&[
            "observe", "metrics", "query", flag, value,
        ]));
        assert!(query.starts_with(&expected), "{query}");
        assert!(!query.contains("candidate_digest="), "{query}");
        assert!(!query.contains("run_id="), "{query}");
        assert!(query.ends_with("}[5m]))"), "{query}");
        assert!(!query.contains("run_id=\"run-abc\""), "{query}");
    }
}
