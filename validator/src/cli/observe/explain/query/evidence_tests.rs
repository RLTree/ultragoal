use super::*;

#[test]
fn evidence_summary_reports_missing_when_receipt_tree_is_absent() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "observe-explain-query-evidence-missing",
    );
    std::fs::create_dir_all(&root).expect("root");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let event = json!({
        "run_id": "run-missing-query-evidence",
        "candidate_digest": candidate,
        "operation": "coverage.prove",
        "failure_class": "coverage_prove_failure"
    });

    let evidence = for_target(&root, Some(&event), &candidate);

    assert_eq!(evidence["logs"]["status"], "missing");
    assert_eq!(evidence["metrics"]["status"], "missing");
    assert_eq!(evidence["traces"]["status"], "missing");
    std::fs::remove_dir_all(root).expect("cleanup missing query evidence");
}

#[test]
fn evidence_summary_matches_top_level_query_failure_class() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "observe-explain-query-evidence-top-level-failure",
    );
    std::fs::create_dir_all(root.join("validation_artifacts/observability/live-loop/commands"))
        .expect("receipt dir");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let receipt =
        root.join("validation_artifacts/observability/live-loop/commands/fmt-traces.json");
    crate::json_boundary::write_json(
        &receipt,
        &json!({
            "schema": crate::cli::observe::command::QUERY_SCHEMA,
            "query_kind": "traces",
            "status": "fail",
            "candidate_digest": candidate,
            "run_id": "run-query-evidence",
            "failure_class": "observability_trace_tree_unavailable",
            "why_failed": "trace lookup returned 404",
            "where_failed": "observe.traces.query",
            "next_repair": "rerun traces query by run and correlation",
            "row_count": 0
        }),
    )
    .expect("query receipt");
    let event = json!({
        "run_id": "run-query-evidence",
        "candidate_digest": candidate,
        "operation": "observe.traces.query",
        "failure_class": "observability_trace_tree_unavailable",
        "receipt_path": "validation_artifacts/observability/live-loop/commands/fmt-traces.json"
    });

    let evidence = for_target(&root, Some(&event), &candidate);
    let direct = target_query_receipt(
        &root,
        &event,
        "traces",
        &candidate,
        "run-query-evidence",
        "observability_trace_tree_unavailable",
    )
    .expect("direct target receipt");

    assert_eq!(evidence["traces"]["status"], "fail");
    assert_eq!(direct["status"], "fail");
    assert_eq!(
        evidence["traces"]["why_failed"],
        "trace lookup returned 404"
    );
    assert_eq!(evidence["logs"]["status"], "missing");
    std::fs::remove_dir_all(root).expect("cleanup query evidence failure");
}

#[test]
fn evidence_summary_keeps_same_run_sibling_queries_for_observation_failure() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "observe-explain-query-evidence-sibling-roundtrips",
    );
    std::fs::create_dir_all(root.join("validation_artifacts/observability/live-loop/commands"))
        .expect("receipt dir");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    write_query_receipt(&root, &candidate, "logs", "pass", "none");
    write_query_receipt(
        &root,
        &candidate,
        "metrics",
        "fail",
        "observability_metric_missing_for_target",
    );
    write_query_receipt(
        &root,
        &candidate,
        "traces",
        "fail",
        "observability_trace_tree_unavailable",
    );
    let event = json!({
        "run_id": "run-sibling-query-evidence",
        "candidate_digest": candidate,
        "operation": "observe.traces.query",
        "failure_class": "observability_trace_tree_unavailable",
        "receipt_path": "validation_artifacts/observability/live-loop/commands/traces-query.json"
    });

    let evidence = for_target(&root, Some(&event), &candidate);

    assert_eq!(evidence["logs"]["status"], "pass");
    assert_eq!(evidence["metrics"]["status"], "fail");
    assert_eq!(evidence["traces"]["status"], "fail");
    assert_eq!(
        evidence["metrics"]["observed_failure_class"],
        "observability_metric_missing_for_target"
    );
    std::fs::remove_dir_all(root).expect("cleanup sibling query evidence");
}

#[test]
fn query_matching_accepts_operation_or_failure_specific_receipts_without_same_run() {
    let metrics = json!({
        "query_kind": "metrics",
        "candidate_digest": "sha256:current",
        "query": "operation=coverage.prove",
        "metric_failure_class": "coverage_prove_failure",
        "metric_error_count": 1
    });
    assert!(query_matches(
        &metrics,
        "metrics",
        "sha256:current",
        "run-current",
        "",
        "coverage.prove",
        "coverage_prove_failure",
    ));

    let pass_target = json!({
        "query_kind": "logs",
        "candidate_digest": "sha256:current",
        "query": "check_id=coverage-current",
        "failure_class": "none"
    });
    assert!(query_matches(
        &pass_target,
        "logs",
        "sha256:current",
        "run-current",
        "coverage-current",
        "",
        "none",
    ));

    let metrics_top_level_failure = json!({
        "query_kind": "metrics",
        "candidate_digest": "sha256:current",
        "query": "operation=coverage.prove",
        "failure_class": "coverage_prove_failure"
    });
    assert!(query_matches(
        &metrics_top_level_failure,
        "metrics",
        "sha256:current",
        "run-current",
        "",
        "coverage.prove",
        "coverage_prove_failure",
    ));
}

#[test]
fn query_matching_rejects_wrong_digest_or_missing_specific_failure_signal() {
    let wrong_digest = json!({
        "query_kind": "logs",
        "candidate_digest": "sha256:old",
        "run_id": "run-current",
        "observed_failure_class": "coverage_prove_failure"
    });
    assert!(!query_matches(
        &wrong_digest,
        "logs",
        "sha256:current",
        "run-current",
        "",
        "",
        "coverage_prove_failure",
    ));

    let missing_failure = json!({
        "query_kind": "traces",
        "candidate_digest": "sha256:current",
        "query": "operation=coverage.prove",
        "failure_class": "none"
    });
    assert!(!query_matches(
        &missing_failure,
        "traces",
        "sha256:current",
        "run-current",
        "",
        "coverage.prove",
        "coverage_prove_failure",
    ));
}

fn write_query_receipt(root: &Path, candidate: &str, kind: &str, status: &str, failure: &str) {
    let receipt = root.join(format!(
        "validation_artifacts/observability/live-loop/commands/{kind}-query.json"
    ));
    crate::json_boundary::write_json(
        &receipt,
        &json!({
            "schema": crate::cli::observe::command::QUERY_SCHEMA,
            "query_kind": kind,
            "status": status,
            "candidate_digest": candidate,
            "run_id": "run-sibling-query-evidence",
            "failure_class": failure,
            "observed_failure_class": failure,
            "why_failed": if status == "pass" { "none" } else { "backend lag" },
            "where_failed": format!("observe.{kind}.query"),
            "next_repair": "rerun target query by run and correlation",
            "row_count": if status == "pass" { 1 } else { 0 }
        }),
    )
    .expect("query receipt");
}
