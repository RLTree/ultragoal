use crate::cli::observe;
use crate::cli::observe::types::ObserveOperation;
use serde_json::json;

#[test]
fn observe_query_helpers_cover_matching_tags_and_bounds() {
    assert_eq!(
        observe::query::query_text(&super::command(&[
            "observe", "logs", "query", "--run-id", "r1"
        ])),
        "run_id:r1"
    );
    assert_eq!(
        observe::query::query_text(&super::command(&[
            "observe", "logs", "query", "--law-id", "law-1"
        ])),
        "law_id:law-1"
    );
    assert_eq!(
        observe::query::query_text(&super::command(&[
            "observe",
            "logs",
            "query",
            "--check-id",
            "check-1"
        ])),
        "check_id:check-1"
    );
    assert_eq!(
        observe::query::query_text(&super::command(&[
            "observe",
            "logs",
            "query",
            "--claim-id",
            "claim-1"
        ])),
        "claim_id:claim-1"
    );
    assert_eq!(
        observe::query::query_text(&super::command(&[
            "observe",
            "logs",
            "query",
            "--run-id",
            "run\"x\ny"
        ])),
        "run_id:runxy"
    );
    assert_eq!(
        observe::query::query_text(&super::command(&["observe", "logs", "query"])),
        "*"
    );
    assert_eq!(
        observe::query::query_text(&super::command(&[
            "observe", "logs", "query", "--query", "custom"
        ])),
        "custom"
    );

    for (operation, kind, subcommand) in [
        (ObserveOperation::LogsQuery, "logs", "logs"),
        (ObserveOperation::MetricsQuery, "metrics", "metrics"),
        (ObserveOperation::TracesQuery, "traces", "traces"),
        (ObserveOperation::Snapshot, "logs", "snapshot"),
        (ObserveOperation::Prove, "logs", "prove"),
    ] {
        assert_eq!(observe::query::kind(operation), kind);
        assert_eq!(operation.subcommand(), subcommand);
    }
    assert_eq!(ObserveOperation::Snapshot.id(), "observe.snapshot");

    assert!(observe::query::has_matches(
        "{}",
        ObserveOperation::LogsQuery
    ));
    assert!(!observe::query::has_matches(
        "",
        ObserveOperation::LogsQuery
    ));
    assert!(observe::query::has_matches(
        r#"{"status":"success","data":{"result":[{"metric":{}}]}}"#,
        ObserveOperation::MetricsQuery
    ));
    assert!(!observe::query::has_matches(
        r#"{"data":{"result":[]}}"#,
        ObserveOperation::MetricsQuery
    ));
    assert!(observe::query::has_matches(
        r#"{"total":1}"#,
        ObserveOperation::TracesQuery
    ));
    assert!(observe::query::has_matches(
        r#"{"data":[{"traceID":"1"}]}"#,
        ObserveOperation::TracesQuery
    ));
    assert!(!observe::query::has_matches(
        r#"{"total":0}"#,
        ObserveOperation::TracesQuery
    ));
    let clipped = observe::query::bounded_rows("abcdef".to_string(), 3);
    assert!(clipped[0]["body"].as_str().unwrap().contains("[truncated]"));
    assert_eq!(
        observe::query::bounded_rows("abc".to_string(), 10)[0]["body"],
        "abc"
    );
    let home_marker = format!("/{}/", "Users");
    let tmp_marker = format!("/{}/{}/", "private", "tmp");
    let private_rows = observe::query::bounded_rows(
        format!(
            r#"{{"path":"{}terrynoblin/Projects/repo/file.json","tmp":"{}proof.json"}}"#,
            home_marker, tmp_marker
        ),
        200,
    );
    let private_body = private_rows[0]["body"].as_str().unwrap();
    assert!(!private_body.contains(&home_marker));
    assert!(!private_body.contains(&tmp_marker));
    assert!(private_body.contains("[redacted-home-path]"));
    assert!(private_body.contains("[redacted-private-tmp-path]"));
    let digest = crate::self_tests::boundaries::support::sha('a');
    assert!(observe::query::candidate_digest_failure("not json", &digest).is_some());
    assert!(observe::query::candidate_digest_failure(
        &format!(
            r#"{{"outer":[{{"candidate_digest":"{digest}"}},{{"candidate_digest":"{digest}"}}],"nested":"{{\"candidate_digest\":\"{digest}\"}}"}}"#
        ),
        &digest,
    )
    .is_none());
    assert!(observe::query::candidate_digest_failure(
        &format!(
            "{{\"candidate_digest\":\"{digest}\"}}\n{{\"nested\":\"{{\\\"candidate_digest\\\":\\\"{digest}\\\"}}\"}}"
        ),
        &digest,
    )
    .is_none());
    assert!(observe::query::candidate_digest_failure(
        r#"{"candidate_digest":"sha256:short"}"#,
        &digest,
    )
    .is_some());
    let observed = observe::query::observed_failure(&[
        json!({"not_body": true}),
        json!({"body": r#"{"rows":[{"status":"fail","why_failed":"nested failure"}]}"#}),
    ])
    .expect("nested observed failure");
    assert_eq!(observed["why_failed"], "nested failure");
    for (raw, expected) in [
        (&["observe", "traces", "query", "--run-id", "r1"][..], "r1"),
        (
            &["observe", "traces", "query", "--law-id", "law-1"][..],
            "law-1",
        ),
        (
            &["observe", "traces", "query", "--check-id", "c1"][..],
            "c1",
        ),
        (
            &["observe", "traces", "query", "--claim-id", "cl1"][..],
            "cl1",
        ),
        (&["observe", "traces", "query"][..], "{}"),
    ] {
        assert!(observe::query::trace_tags(&super::command(raw)).contains(expected));
    }
}
