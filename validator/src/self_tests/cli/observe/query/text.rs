use crate::cli::observe;

#[test]
fn observe_metric_query_text_covers_all_authority_selectors() {
    for (flag, value, expected) in [
        (
            "--run-id",
            "run-abc",
            r#"ultragoal_command_total{run_id="run-abc"}"#,
        ),
        (
            "--law-id",
            "law-abc",
            r#"ultragoal_command_total{law_id="law-abc"}"#,
        ),
        (
            "--check-id",
            "check-abc",
            r#"ultragoal_command_total{check_id="check-abc"}"#,
        ),
        (
            "--claim-id",
            "claim-abc",
            r#"ultragoal_command_total{claim_id="claim-abc"}"#,
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
