use crate::cli::observe;
use serde_json::{Value, json};
use std::fs;

#[test]
fn observe_snapshot_refuses_semantically_empty_explain_proof() {
    let root = super::super::minimal_root("observe-snapshot-empty-explain");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let target = super::write_source_audit_receipt(&root, &candidate);
    super::write_query_receipts(&root, &candidate, &target);
    super::write_explain_receipt(&root, &candidate, &target, false);
    let explain_path =
        root.join("validation_artifacts/observability/source-audit-explain-failure.json");
    let mut explain = crate::json_boundary::read_json(&explain_path).expect("explain");
    remove_field(&mut explain, "/observed_why_failed");
    remove_field(&mut explain, "/explanation/implicated_paths");
    crate::json_boundary::write_json(&explain_path, &explain).expect("write explain");

    let proof_rel = "validation_artifacts/observability/source-audit-production-proof.json";
    let command = super::command(&[
        "observe",
        "snapshot",
        "--run-id",
        &target.run_id,
        "--receipt",
        proof_rel,
    ]);
    assert_eq!(observe::run(&root, &command).expect("snapshot"), 1);
    let proof = crate::json_boundary::read_json(&root.join(proof_rel)).expect("proof");
    let why = proof["why_failed"].as_str().unwrap();
    assert!(
        why.contains("explain_failure_missing_why_failed"),
        "{proof}"
    );
    assert!(
        proof["explain_evidence"]["implicated_paths"]
            .as_array()
            .is_some_and(Vec::is_empty),
        "{proof}"
    );
    fs::remove_dir_all(root).expect("cleanup snapshot empty explain");
}

#[test]
fn observe_snapshot_accepts_pass_target_explain_without_failure_where_why() {
    let root = super::super::minimal_root("observe-snapshot-pass-explain");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let target = write_review_target_pass_receipt(&root, &candidate);
    write_pass_queries(&root, &candidate, &target);
    write_pass_explain(&root, &candidate, &target);

    let proof_rel = "validation_artifacts/observability/review-target-production-proof.json";
    let command = super::command(&[
        "observe",
        "snapshot",
        "--run-id",
        &target.run_id,
        "--receipt",
        proof_rel,
    ]);
    assert_eq!(observe::run(&root, &command).expect("snapshot"), 0);
    let proof = crate::json_boundary::read_json(&root.join(proof_rel)).expect("proof");
    assert_eq!(proof["status"], "pass");
    assert_eq!(proof["explain_evidence"]["target_status"], "pass");
    assert_eq!(proof["explain_evidence"]["where_failed"], "none");
    assert_eq!(proof["explain_evidence"]["why_failed"], "none");
    fs::remove_dir_all(root).expect("cleanup snapshot pass explain");
}

fn remove_field(value: &mut Value, pointer: &str) {
    let (parent, field) = pointer.rsplit_once('/').expect("pointer");
    value
        .pointer_mut(parent)
        .and_then(Value::as_object_mut)
        .expect("object")
        .remove(field);
}

fn write_review_target_pass_receipt(root: &std::path::Path, candidate: &str) -> super::TargetIds {
    let receipt = observe::telemetry::command_receipt_for_candidate(
        root,
        observe::telemetry::CommandTelemetry {
            command: "ultragoal review-target",
            subcommand: "build",
            operation: "review-target.build",
            surface: "review_target",
            law_id: observe::types::LAW_ID,
            check_id: "review-target-build-observability-binding",
            claim_id: "review_target_source_local_observability",
            artifact_path: "validation_artifacts/review/review-target-receipt.json",
            receipt_path: "validation_artifacts/observability/review-target-build.json",
            status: "pass",
            failure_class: "none",
            why_failed: "none",
            where_failed: "none",
            next_repair: "none",
            claim_impact: "supports_review_target_source_local_observability_only",
            blocked_claims: vec![],
            supported_claims: vec!["review_target_source_local_observability".to_string()],
            runtime: None,
            emit: false,
        },
        candidate.to_string(),
    )
    .expect("review target receipt");
    let run_id = super::text(&receipt, "run_id");
    let correlation_id = super::text(&receipt, "correlation_id");
    crate::json_boundary::write_json(
        &root.join("validation_artifacts/observability/review-target-build.json"),
        &receipt,
    )
    .expect("write review target receipt");
    super::TargetIds {
        run_id,
        correlation_id,
    }
}

fn write_pass_queries(root: &std::path::Path, candidate: &str, target: &super::TargetIds) {
    for kind in ["logs", "metrics", "traces"] {
        let mut value = json!({
            "schema": observe::types::QUERY_SCHEMA,
            "status": "pass",
            "candidate_digest": candidate,
            "run_id": target.run_id,
            "correlation_id": target.correlation_id,
            "query_kind": kind,
            "query": format!("run_id:{} operation:review-target.build", target.run_id),
            "row_count": 1,
            "observed_failure_class": "none",
            "observed_why_failed": "none",
            "observed_where_failed": "none",
            "observed_next_repair": "none"
        });
        if kind == "metrics" {
            value["metric_failure_class"] = json!("none");
            value["metric_error_count"] = json!(0);
        }
        crate::json_boundary::write_json(
            &root.join(format!(
                "validation_artifacts/observability/review-target-build-{kind}-query.json"
            )),
            &value,
        )
        .expect("pass query receipt");
    }
}

fn write_pass_explain(root: &std::path::Path, candidate: &str, target: &super::TargetIds) {
    crate::json_boundary::write_json(
        &root.join("validation_artifacts/observability/review-target-build-explain-failure.json"),
        &json!({
            "schema": observe::types::RECEIPT_SCHEMA,
            "status": "pass",
            "candidate_digest": candidate,
            "operation": "observe.explain-failure",
            "run_id": target.run_id,
            "correlation_id": target.correlation_id,
            "observed_run": {
                "operation": "review-target.build",
                "status": "pass"
            },
            "explanation": {
                "broad_rerun": "source audit once if required",
                "claim_ceiling": "source-local only",
                "fallback_used": false,
                "implicated_paths": [
                    "plugin-manifest-draft.json",
                    "validation_artifacts/review/review-target-receipt.json",
                    "validation_artifacts/observability/review-target-build.json"
                ],
                "query_evidence": {
                    "logs": {"where_failed": "none", "why_failed": "none"},
                    "metrics": {"where_failed": "none", "why_failed": "none"},
                    "traces": {"where_failed": "none", "why_failed": "none"}
                },
                "root_cause": "requested telemetry target passed; no failure",
                "smallest_repair": "No repair for requested telemetry target",
                "narrow_rerun": "ultragoal review-target build"
            }
        }),
    )
    .expect("pass explain receipt");
}
