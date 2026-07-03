use crate::cli::observe;
use crate::cli::observe::types::{QUERY_SCHEMA, RECEIPT_SCHEMA};
use serde_json::{Value, json};
use std::fs;

mod edges;
mod explain_edges;
mod target_edges;

#[test]
fn observe_snapshot_writes_stable_target_command_roundtrip() {
    let root = super::minimal_root("observe-snapshot-source-audit-proof");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let target = write_source_audit_receipt(&root, &candidate);
    write_query_receipts(&root, &candidate, &target);
    write_explain_receipt(&root, &candidate, &target, false);

    let proof_rel = "validation_artifacts/observability/source-audit-command-roundtrip.json";
    let command = command(&[
        "observe",
        "snapshot",
        "--run-id",
        &target.run_id,
        "--receipt",
        proof_rel,
    ]);
    assert_eq!(observe::run(&root, &command).expect("snapshot"), 0);
    let proof = crate::json_boundary::read_json(&root.join(proof_rel)).expect("proof");

    assert_eq!(
        proof["schema"],
        "harness-ultragoal.observability-command-roundtrip.v1"
    );
    assert_eq!(proof["status"], "pass");
    assert_eq!(proof["candidate_digest"], candidate);
    assert_eq!(proof["target_operation"], "source.audit");
    assert_eq!(proof["observability"]["operation"], "source.audit");
    assert_eq!(proof["observability"]["run_id"], target.run_id);
    assert_eq!(proof["query_evidence"]["logs"]["status"], "pass");
    assert!(
        proof["query_evidence"]["logs"]["why_failed"]
            .as_str()
            .unwrap()
            .contains("source audit failed checks"),
        "{proof}"
    );
    assert_eq!(
        proof["query_evidence"]["logs"]["next_repair"],
        "repair agent-standards stale fit-repo receipt"
    );
    assert_eq!(proof["query_evidence"]["metrics"]["status"], "pass");
    assert_eq!(proof["query_evidence"]["traces"]["status"], "pass");
    assert_eq!(proof["explain_evidence"]["status"], "pass");
    assert_eq!(proof["explain_evidence"]["fallback_used"], false);
    assert!(
        proof["same_candidate_query_proof_paths"]
            .as_array()
            .unwrap()
            .iter()
            .all(|path| path.as_str().unwrap().starts_with("validation_artifacts/")),
        "{proof}"
    );
    assert!(
        !proof.to_string().contains(root.to_string_lossy().as_ref()),
        "{proof}"
    );
    fs::remove_dir_all(root).expect("cleanup snapshot proof");
}

#[test]
fn observe_snapshot_refuses_to_fit_without_non_fallback_explain() {
    let root = super::minimal_root("observe-snapshot-missing-explain");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let target = write_source_audit_receipt(&root, &candidate);
    write_query_receipts(&root, &candidate, &target);

    let proof_rel = "validation_artifacts/observability/source-audit-command-roundtrip.json";
    let command = command(&[
        "observe",
        "snapshot",
        "--run-id",
        &target.run_id,
        "--receipt",
        proof_rel,
    ]);
    assert_eq!(observe::run(&root, &command).expect("snapshot"), 1);
    let proof = crate::json_boundary::read_json(&root.join(proof_rel)).expect("proof");
    assert_eq!(proof["status"], "fail");
    assert!(
        proof["why_failed"]
            .as_str()
            .unwrap()
            .contains("same_candidate_explain_failure_not_proven"),
        "{proof}"
    );
    assert_eq!(proof["supported_claims"], json!([]));

    write_explain_receipt(&root, &candidate, &target, true);
    assert_eq!(observe::run(&root, &command).expect("snapshot"), 1);
    let fallback = crate::json_boundary::read_json(&root.join(proof_rel)).expect("fallback");
    assert!(
        fallback["why_failed"]
            .as_str()
            .unwrap()
            .contains("explain_failure_used_fallback"),
        "{fallback}"
    );
    fs::remove_dir_all(root).expect("cleanup snapshot missing explain");
}

pub(super) struct TargetIds {
    pub(super) run_id: String,
    pub(super) correlation_id: String,
}

pub(super) fn command(raw: &[&str]) -> observe::types::ObserveCommand {
    observe::parse(&super::args(raw))
        .expect("parse")
        .expect("observe command")
}

pub(super) fn write_source_audit_receipt(root: &std::path::Path, candidate: &str) -> TargetIds {
    let receipt = observe::telemetry::command_receipt_for_candidate(
        root,
        observe::telemetry::CommandTelemetry {
            command: "ultragoal source",
            subcommand: "audit",
            operation: "source.audit",
            surface: "source",
            law_id: observe::types::LAW_ID,
            check_id: "source-audit-observability-binding",
            claim_id: "source_audit",
            artifact_path: "validation_artifacts/ultragoal-audit",
            receipt_path: "validation_artifacts/observability/source-audit.json",
            status: "fail",
            failure_class: "source_audit_check_failure",
            why_failed: "source audit failed checks: total_failures=1 first_check=agent-standards root_group=source_audit_check_failure first_detail=fit_repo_receipt_target_digest_mismatch",
            where_failed: "source.audit",
            next_repair: "query this run through observe logs/metrics/traces, repair agent-standards stale fit-repo receipt, then rerun source audit once",
            claim_impact: "source_audit_failed_blocks_readiness_release_completion_update_goal",
            blocked_claims: vec![
                "completion".to_string(),
                "readiness".to_string(),
                "release".to_string(),
                "reviewer_exposure".to_string(),
                "final_packet_correctness".to_string(),
                "update_goal_eligibility".to_string(),
            ],
            supported_claims: vec![],
            runtime: None,
            emit: false,
        },
        candidate.to_string(),
    )
    .expect("source audit receipt");
    let run_id = text(&receipt, "run_id");
    let correlation_id = text(&receipt, "correlation_id");
    crate::json_boundary::write_json(
        &root.join("validation_artifacts/observability/source-audit.json"),
        &receipt,
    )
    .expect("write source audit receipt");
    TargetIds {
        run_id,
        correlation_id,
    }
}

pub(super) fn write_query_receipts(root: &std::path::Path, candidate: &str, target: &TargetIds) {
    for kind in ["logs", "metrics", "traces"] {
        let mut value = json!({
            "schema": QUERY_SCHEMA,
            "status": "pass",
            "candidate_digest": candidate,
            "run_id": target.run_id,
            "correlation_id": target.correlation_id,
            "query_kind": kind,
            "query": format!("run_id:{} operation:source.audit", target.run_id),
            "row_count": 1,
            "observed_failure_class": "source_audit_check_failure",
            "observed_why_failed": "source audit failed checks: total_failures=1 first_check=agent-standards",
            "observed_where_failed": "source.audit",
            "observed_next_repair": "repair agent-standards stale fit-repo receipt",
            "rows": [{
                "candidate_digest": candidate,
                "correlation_id": target.correlation_id,
                "operation": "source.audit",
                "failure_class": "source_audit_check_failure",
                "why_failed": "source audit failed checks: total_failures=1 first_check=agent-standards"
            }]
        });
        if kind == "metrics" {
            value["metric_failure_class"] = json!("source_audit_check_failure");
            value["metric_error_count"] = json!(1);
        }
        crate::json_boundary::write_json(
            &root.join(format!(
                "validation_artifacts/observability/source-audit-{kind}-query.json"
            )),
            &value,
        )
        .expect("query receipt");
    }
}

pub(super) fn write_explain_receipt(
    root: &std::path::Path,
    candidate: &str,
    target: &TargetIds,
    fallback_used: bool,
) {
    crate::json_boundary::write_json(
        &root.join("validation_artifacts/observability/source-audit-explain-failure.json"),
        &json!({
            "schema": RECEIPT_SCHEMA,
            "status": "pass",
            "candidate_digest": candidate,
            "operation": "observe.explain-failure",
            "run_id": target.run_id,
            "correlation_id": target.correlation_id,
            "observed_why_failed": "source audit failed checks: total_failures=1 first_check=agent-standards",
            "observed_where_failed": "source.audit",
            "observed_run": {
                "operation": "source.audit",
                "status": "fail"
            },
            "explanation": {
                "broad_rerun": "source audit once",
                "claim_ceiling": "source-local only",
                "fallback_used": fallback_used,
                "implicated_paths": ["validation_artifacts/observability/source-audit.json"],
                "root_cause": "stale fit-repo receipt blocks source audit",
                "smallest_repair": "rebind fit-repo receipt for the current candidate",
                "narrow_rerun": "source audit once after rebind"
            }
        }),
    )
    .expect("explain receipt");
}

pub(super) fn text(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_str)
        .expect(field)
        .to_string()
}
