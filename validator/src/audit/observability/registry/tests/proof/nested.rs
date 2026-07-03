use super::super::super::proof;
use super::super::inventory_fixtures::{write_registry_root, write_valid_fixture};
use serde_json::json;

#[test]
fn command_inventory_accepts_nested_law_receipt_observability_binding() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("observe-nested-receipt");
    write_registry_root(
        &root,
        super::super::inventory_fixtures::observable_inventory(),
    );
    write_valid_fixture(&root);
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let run = "run-final-packet";
    let corr = "corr-final-packet";
    let receipt = "validation_artifacts/review/final-packet-proof.json";
    write_nested_receipt(&root, &candidate, run, corr, receipt);
    for kind in ["logs", "metrics", "traces"] {
        write_query(&root, &candidate, run, corr, kind);
    }
    write_explain(&root, &candidate, run, corr);

    let row = json!({
        "receipt_paths": [receipt],
        "same_candidate_query_proof_paths": [
            "validation_artifacts/observability/final-packet-prove-logs-query.json",
            "validation_artifacts/observability/final-packet-prove-metrics-query.json",
            "validation_artifacts/observability/final-packet-prove-traces-query.json",
            "validation_artifacts/observability/final-packet-prove-explain-failure.json"
        ]
    });
    let mut failures = Vec::new();
    proof::require_current_receipts(
        &root,
        "final-packet prove",
        row.as_object().unwrap(),
        &mut failures,
    );
    assert!(failures.is_empty(), "{failures:?}");

    let mut stale = crate::json_boundary::read_json(&root.join(receipt)).expect("receipt");
    stale["observability"]["blocked_claims"] = json!(["completion"]);
    crate::json_boundary::write_json(&root.join(receipt), &stale).expect("stale receipt");
    proof::require_current_receipts(
        &root,
        "final-packet prove",
        row.as_object().unwrap(),
        &mut failures,
    );
    assert!(
        failures.iter().any(|item| {
            item.starts_with(
                "observability_command_telemetry_receipt_not_current:final-packet prove",
            )
        }),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn nested_command_inventory_rejects_high_cardinality_metric_labels() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "observe-nested-metric-labels",
    );
    write_registry_root(
        &root,
        super::super::inventory_fixtures::observable_inventory(),
    );
    write_valid_fixture(&root);
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let run = "run-final-packet";
    let corr = "corr-final-packet";
    let receipt = "validation_artifacts/review/final-packet-proof.json";
    write_nested_receipt(&root, &candidate, run, corr, receipt);
    for kind in ["logs", "metrics", "traces"] {
        write_query(&root, &candidate, run, corr, kind);
    }
    let metrics_rel = "validation_artifacts/observability/final-packet-prove-metrics-query.json";
    let mut metrics = crate::json_boundary::read_json(&root.join(metrics_rel)).expect("metrics");
    metrics["rows"][0]["metric"]["run_id"] = json!(run);
    crate::json_boundary::write_json(&root.join(metrics_rel), &metrics).expect("metrics write");
    write_explain(&root, &candidate, run, corr);

    let row = json!({
        "receipt_paths": [receipt],
        "same_candidate_query_proof_paths": [
            "validation_artifacts/observability/final-packet-prove-logs-query.json",
            metrics_rel,
            "validation_artifacts/observability/final-packet-prove-traces-query.json",
            "validation_artifacts/observability/final-packet-prove-explain-failure.json"
        ]
    });
    let mut failures = Vec::new();
    proof::require_current_receipts(
        &root,
        "final-packet prove",
        row.as_object().unwrap(),
        &mut failures,
    );
    assert!(
        failures.iter().any(|item| item
            .starts_with("observability_command_telemetry_query_not_same_run:final-packet prove")),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

fn write_nested_receipt(root: &std::path::Path, candidate: &str, run: &str, corr: &str, rel: &str) {
    crate::json_boundary::write_json(
        &root.join(rel),
        &json!({
            "schema": "harness-ultragoal.final-packet-proof.v1",
            "status": "fail",
            "candidate_digest": candidate,
            "run_id": run,
            "correlation_id": corr,
            "observability": {
                "schema": crate::cli::observe::types::RECEIPT_SCHEMA,
                "status": "fail",
                "candidate_digest": candidate,
                "operation": "final-packet.prove",
                "run_id": run,
                "correlation_id": corr,
                "claim_ceiling": "withheld_or_blocked",
                "claim_impact": "final_packet_correctness_review_readiness_release_completion_update_goal_blocked",
                "supported_claims": [],
                "blocked_claims": [
                    "completion",
                    "package_readiness",
                    "review_readiness",
                    "release_readiness",
                    "update_goal_eligibility",
                    "app_registry_or_reviewer_exposure"
                ]
            }
        }),
    )
    .expect("nested receipt");
}

fn write_query(root: &std::path::Path, candidate: &str, run: &str, corr: &str, kind: &str) {
    crate::json_boundary::write_json(
        &root.join(format!(
            "validation_artifacts/observability/final-packet-prove-{kind}-query.json"
        )),
        &json!({
            "schema": crate::cli::observe::types::QUERY_SCHEMA,
            "status": "pass",
            "candidate_digest": candidate,
            "run_id": run,
            "correlation_id": corr,
            "query_kind": kind,
            "rows": [{
                "candidate_digest": candidate,
                "operation": "final-packet.prove",
                "correlation_id": corr,
                "metric": {"__name__": "ultragoal_command_total", "operation": "final-packet.prove"}
            }]
        }),
    )
    .expect("query receipt");
}

fn write_explain(root: &std::path::Path, candidate: &str, run: &str, corr: &str) {
    crate::json_boundary::write_json(
        &root.join("validation_artifacts/observability/final-packet-prove-explain-failure.json"),
        &json!({
            "schema": crate::cli::observe::types::RECEIPT_SCHEMA,
            "status": "pass",
            "candidate_digest": candidate,
            "operation": "observe.explain-failure",
            "run_id": run,
            "correlation_id": corr
        }),
    )
    .expect("explain receipt");
}
