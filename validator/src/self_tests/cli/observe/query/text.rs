use crate::cli::observe;

#[test]
fn observe_metric_query_text_covers_all_authority_selectors() {
    for (flag, value, expected) in [
        (
            "--run-id",
            "run-abc",
            r#"count_over_time(ultragoal_command_total{run_id="run-abc"}[2h])"#,
        ),
        (
            "--law-id",
            "law-abc",
            r#"count_over_time(ultragoal_command_total{law_id="law-abc"}[2h])"#,
        ),
        (
            "--check-id",
            "check-abc",
            r#"count_over_time(ultragoal_command_total{check_id="check-abc"}[2h])"#,
        ),
        (
            "--claim-id",
            "claim-abc",
            r#"count_over_time(ultragoal_command_total{claim_id="claim-abc"}[2h])"#,
        ),
    ] {
        assert_eq!(
            observe::query::query_text(&super::command(&[
                "observe", "metrics", "query", flag, value,
            ])),
            expected
        );
    }
}
