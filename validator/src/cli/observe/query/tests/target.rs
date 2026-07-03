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
fn requested_selector_prefers_run_then_correlation_check_claim_law() {
    let mut command = selector_command();
    assert_eq!(
        super::super::target::requested(&command),
        Some("run_id=run-target".to_string())
    );
    command.run_id = None;
    assert_eq!(
        super::super::target::requested(&command),
        Some("correlation_id=corr-target".to_string())
    );
    command.correlation_id = None;
    assert_eq!(
        super::super::target::requested(&command),
        Some("check_id=check-target".to_string())
    );
    command.check_id = None;
    assert_eq!(
        super::super::target::requested(&command),
        Some("claim_id=claim-target".to_string())
    );
    command.claim_id = None;
    assert_eq!(
        super::super::target::requested(&command),
        Some("law_id=law-target".to_string())
    );
    command.law_id = None;
    assert_eq!(super::super::target::requested(&command), None);
}

#[test]
fn target_event_uses_latest_non_observation_spool_event() {
    let root = super::prepare_root("observe-query-target-spool");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let target = json!({
        "schema": crate::cli::observe::types::EVENT_SCHEMA,
        "run_id": "run-target",
        "correlation_id": "corr-target",
        "candidate_digest": candidate,
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
    });
    crate::cli::observe::telemetry::spool_write_for_test(Path::new(&root), &target)
        .expect("target event");
    let observation = json!({
        "schema": crate::cli::observe::types::EVENT_SCHEMA,
        "run_id": "run-target",
        "correlation_id": "corr-target",
        "operation": "observe.logs.query",
        "candidate_digest": candidate
    });
    crate::cli::observe::telemetry::spool_write_for_test(Path::new(&root), &observation)
        .expect("observation event");

    let found =
        super::super::target::event(Path::new(&root), &selector_command()).expect("target event");

    assert_eq!(found["operation"], "source.audit");
    assert_eq!(found["failure_class"], "source_audit_check_failure");
    std::fs::remove_dir_all(root).expect("cleanup target spool");
}

#[test]
fn target_event_with_missing_operation_is_not_misclassified_as_query_observation() {
    let root = super::prepare_root("observe-query-target-no-operation");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    crate::cli::observe::telemetry::spool_write_for_test(
        Path::new(&root),
        &json!({
            "schema": crate::cli::observe::types::EVENT_SCHEMA,
            "run_id": "run-target",
            "correlation_id": "corr-target",
            "candidate_digest": candidate,
            "status": "fail",
            "failure_class": "coverage_prove_failure",
            "law_id": "law-target",
            "check_id": "check-target",
            "claim_id": "claim-target"
        }),
    )
    .expect("target event");

    let found = super::super::target::event(Path::new(&root), &selector_command())
        .expect("target without operation");

    assert_eq!(found["failure_class"], "coverage_prove_failure");
    assert!(found.get("operation").is_none());
    std::fs::remove_dir_all(root).expect("cleanup no operation target");
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
