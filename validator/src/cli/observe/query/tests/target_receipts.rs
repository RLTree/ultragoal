use crate::cli::observe::types::{ObserveCommand, ObserveOperation};
use serde_json::json;
use std::path::Path;

fn selector_command() -> ObserveCommand {
    ObserveCommand {
        operation: ObserveOperation::MetricsQuery,
        receipt: None,
        query: None,
        run_id: Some("run-target".to_string()),
        correlation_id: Some("corr-target".to_string()),
        claim_id: Some("claim-target".to_string()),
        check_id: Some("check-target".to_string()),
        law_id: Some("law-target".to_string()),
        target_command: None,
        target_family: None,
        row_limit: 100,
        byte_limit: 4096,
        timeout_ms: 100,
    }
}

#[test]
fn target_event_skips_query_receipts_and_fails_closed_for_receipt_without_event() {
    let root = super::prepare_root("observe-query-target-receipt");
    let dir = root.join("validation_artifacts/observability");
    std::fs::create_dir_all(&dir).expect("observability dir");
    crate::json_boundary::write_json(
        &dir.join("z-query.json"),
        &json!({
            "schema": crate::cli::observe::types::QUERY_SCHEMA,
            "run_id": "run-target",
            "operation": "observe.logs.query"
        }),
    )
    .expect("query receipt");
    crate::json_boundary::write_json(
        &dir.join("a-target.json"),
        &json!({
            "schema": crate::cli::observe::types::RECEIPT_SCHEMA,
            "run_id": "run-target",
            "correlation_id": "corr-target",
            "candidate_digest": "sha256:receipt",
            "operation": "coverage.prove",
            "status": "fail",
            "failure_class": "coverage_prove_failure",
            "why_failed": "coverage receipt is stale",
            "where_failed": "coverage.manifest_digest",
            "next_repair": "rerun coverage prove",
            "claim_impact": "coverage_blocks_readiness",
            "law_id": "law-target",
            "check_id": "check-target",
            "claim_id": "claim-target"
        }),
    )
    .expect("target receipt");

    let found =
        super::super::target::event(Path::new(&root), &selector_command()).expect("receipt event");

    assert_eq!(found["operation"], "coverage.prove");
    assert_eq!(found["status"], "fail");
    assert_eq!(
        found["failure_class"],
        "receipt_without_observability_event"
    );
    assert!(
        found["why_failed"]
            .as_str()
            .unwrap()
            .contains("has no event object")
    );
    assert_eq!(
        found["where_failed"],
        "observe.target.receipt_event_binding"
    );
    assert!(
        found["next_repair"]
            .as_str()
            .unwrap()
            .contains("real event emission")
    );
    assert_eq!(
        found["claim_impact"],
        "observability_reconciliation_blocked"
    );
    assert_eq!(found["observed_receipt_status"], "fail");
    assert_eq!(found["fallback_only"], true);
    std::fs::remove_dir_all(root).expect("cleanup target receipt");
}

#[test]
fn target_event_reads_nested_command_observation_receipts() {
    let root = super::prepare_root("observe-query-target-nested-receipt");
    let dir = root.join("validation_artifacts/observability/live-loop/commands");
    std::fs::create_dir_all(&dir).expect("nested observability dir");
    crate::json_boundary::write_json(
        &dir.join("fmt-check-command-observation.json"),
        &json!({
            "schema": crate::cli::observe::types::RECEIPT_SCHEMA,
            "run_id": "run-target",
            "correlation_id": "corr-target",
            "candidate_digest": "sha256:receipt",
            "status": "pass",
            "event": {
                "run_id": "run-target",
                "correlation_id": "corr-target",
                "candidate_digest": "sha256:receipt",
                "operation": "loop.measure.fmt_check",
                "status": "pass",
                "failure_class": "none",
                "why_failed": "none",
                "where_failed": "none",
                "next_repair": "none",
                "claim_impact": "source_local_live_loop_node_observation_only_not_speed_claim",
                "law_id": "law-target",
                "check_id": "check-target",
                "claim_id": "claim-target"
            }
        }),
    )
    .expect("nested target receipt");

    let found = super::super::target::event(Path::new(&root), &selector_command())
        .expect("nested command observation event");

    assert_eq!(found["operation"], "loop.measure.fmt_check");
    assert_eq!(found["status"], "pass");
    assert!(found.get("fallback_only").is_none());
    std::fs::remove_dir_all(root).expect("cleanup nested target receipt");
}

#[test]
fn target_event_skips_bad_spool_lines_and_prefers_receipt_event_binding() {
    let root = super::prepare_root("observe-query-target-bad-spool-and-receipt");
    let dir = root.join("validation_artifacts/observability");
    std::fs::create_dir_all(&dir).expect("observability dir");
    std::fs::create_dir_all(dir.join("spool")).expect("spool dir");
    std::fs::write(dir.join("spool/events.jsonl"), "{not-json}\n").expect("bad spool line");
    std::fs::write(dir.join("z-bad.json"), "{not-json").expect("bad receipt");
    crate::json_boundary::write_json(
        &dir.join("a-target-event.json"),
        &json!({
            "schema": crate::cli::observe::types::RECEIPT_SCHEMA,
            "run_id": "run-target",
            "correlation_id": "corr-target",
            "candidate_digest": "sha256:receipt",
            "status": "fail",
            "event": {
                "run_id": "run-target",
                "correlation_id": "corr-target",
                "candidate_digest": "sha256:receipt",
                "operation": "source.audit",
                "status": "fail",
                "failure_class": "source_audit_check_failure",
                "why_failed": "coverage receipt stale",
                "where_failed": "source_audit.coverage",
                "next_repair": "rerun exact coverage",
                "claim_impact": "readiness_release_completion_update_goal_blocked",
                "law_id": "law-target",
                "check_id": "check-target",
                "claim_id": "claim-target"
            }
        }),
    )
    .expect("target receipt with event");

    let found =
        super::super::target::event(Path::new(&root), &selector_command()).expect("target event");

    assert_eq!(found["operation"], "source.audit");
    assert_eq!(found["failure_class"], "source_audit_check_failure");
    assert!(found.get("fallback_only").is_none());
    std::fs::remove_dir_all(root).expect("cleanup target receipt event");
}
