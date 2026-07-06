use crate::cli::observe;

#[test]
fn observe_metric_query_text_covers_all_authority_selectors() {
    for (flag, value, expected) in [
        (
            "--run-id",
            "run-abc",
            "sum by (__name__,operation,status,check_id,claim_id,surface,failure_class,exporter,saturation_status) (last_over_time({__name__=~\"ultragoal_command_total|ultragoal_command_duration_ms|ultragoal_command_task_count|ultragoal_command_queue_depth\"",
        ),
        (
            "--law-id",
            "law-abc",
            r#"sum by (__name__,operation,status,check_id,claim_id,surface,failure_class,exporter,saturation_status) (last_over_time({__name__=~"ultragoal_command_total|ultragoal_command_duration_ms|ultragoal_command_task_count|ultragoal_command_queue_depth",law_id="law-abc""#,
        ),
        (
            "--check-id",
            "check-abc",
            r#"sum by (__name__,operation,status,check_id,claim_id,surface,failure_class,exporter,saturation_status) (last_over_time({__name__=~"ultragoal_command_total|ultragoal_command_duration_ms|ultragoal_command_task_count|ultragoal_command_queue_depth",check_id="check-abc""#,
        ),
        (
            "--claim-id",
            "claim-abc",
            r#"sum by (__name__,operation,status,check_id,claim_id,surface,failure_class,exporter,saturation_status) (last_over_time({__name__=~"ultragoal_command_total|ultragoal_command_duration_ms|ultragoal_command_task_count|ultragoal_command_queue_depth",claim_id="claim-abc""#,
        ),
    ] {
        let query = observe::query::query_text(&super::command(&[
            "observe", "metrics", "query", flag, value,
        ]));
        assert!(query.starts_with(expected), "{query}");
        assert!(!query.contains("candidate_digest="), "{query}");
        assert!(!query.contains("run_id="), "{query}");
        assert!(query.ends_with("}[5m]))"), "{query}");
        assert!(!query.contains("run_id=\"run-abc\""), "{query}");
    }
}
