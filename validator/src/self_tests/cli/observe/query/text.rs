use crate::cli::observe;

#[test]
fn observe_metric_query_text_covers_all_authority_selectors() {
    for (flag, value, expected) in [
        (
            "--run-id",
            "run-abc",
            "max_over_time(ultragoal_command_total{",
        ),
        (
            "--law-id",
            "law-abc",
            r#"max_over_time(ultragoal_command_total{law_id="law-abc","#,
        ),
        (
            "--check-id",
            "check-abc",
            r#"max_over_time(ultragoal_command_total{check_id="check-abc","#,
        ),
        (
            "--claim-id",
            "claim-abc",
            r#"max_over_time(ultragoal_command_total{claim_id="claim-abc","#,
        ),
    ] {
        let query = observe::query::query_text(&super::command(&[
            "observe", "metrics", "query", flag, value,
        ]));
        assert!(query.starts_with(expected), "{query}");
        assert!(query.contains("candidate_digest=\"\""), "{query}");
        assert!(query.contains("run_id=\"\""), "{query}");
        assert!(query.ends_with("}[24h])"), "{query}");
        assert!(!query.contains("run_id=\"run-abc\""), "{query}");
    }
}
