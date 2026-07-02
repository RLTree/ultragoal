use crate::cli::observe::types::ObserveCommand;
use serde_json::{Value, json};
use std::path::Path;

mod explain;
mod explain_validation;
mod target;

use target::TargetReceipt;

const PROOF_SCHEMA: &str = "harness-ultragoal.observability-production-proof.v1";

pub(crate) fn run(root: &Path, command: &ObserveCommand) -> Result<Value, String> {
    let candidate = crate::package::inventory::package_digest(root)?;
    let selectors = target::selectors(command);
    let target = if selectors.is_empty() {
        None
    } else {
        target::find_target_receipt(root, command)
    };
    let mut failures = Vec::new();
    let query_evidence = target
        .as_ref()
        .map(|target| {
            target::validate_target_event(&target.event, &candidate, &mut failures);
            super::explain::query_evidence_for_target(root, Some(&target.event), &candidate)
        })
        .unwrap_or_else(|| {
            failures.push(target_missing_failure(&selectors));
            super::explain::query_evidence_for_target(root, None, &candidate)
        });
    validate_query_evidence(&query_evidence, &mut failures);
    let explain_evidence = target
        .as_ref()
        .map(|target| explain::evidence(root, &candidate, &target.event))
        .unwrap_or_else(|| json!({"status":"missing"}));
    explain_validation::validate(&explain_evidence, &mut failures);
    let failure_summary = failure_summary(&failures);
    let status = if failures.is_empty() { "pass" } else { "fail" };
    let snapshot_observability =
        super::telemetry::base_receipt(root, command, status, failure_summary.as_deref())?;
    let mut proof = proof_receipt(
        command,
        status,
        &candidate,
        &snapshot_observability,
        target.as_ref(),
        query_evidence,
        explain_evidence,
        failure_summary,
    );
    if let Some(target) = target {
        proof["observability"] = target.receipt;
    }
    Ok(proof)
}

fn proof_receipt(
    command: &ObserveCommand,
    status: &str,
    candidate: &str,
    snapshot_observability: &Value,
    target: Option<&TargetReceipt>,
    query_evidence: Value,
    explain_evidence: Value,
    failure_summary: Option<String>,
) -> Value {
    let target_event = target.map(|target| &target.event);
    let target_run = target::text(target_event, "run_id", "unknown");
    let target_correlation = target::text(target_event, "correlation_id", "unknown");
    let target_operation = target::text(target_event, "operation", "unknown");
    let query_paths = query_proof_paths(&query_evidence, &explain_evidence);
    json!({
        "schema": PROOF_SCHEMA,
        "status": status,
        "candidate_digest": candidate,
        "run_id": snapshot_observability["run_id"],
        "correlation_id": snapshot_observability["correlation_id"],
        "trace_id": snapshot_observability["trace_id"],
        "operation": "observe.snapshot",
        "target_operation": target_operation,
        "target_run_id": target_run,
        "target_correlation_id": target_correlation,
        "target_receipt_path": target
            .map(|target| target.receipt_rel.clone())
            .unwrap_or_else(|| "missing".to_string()),
        "target_status": target::text(target_event, "status", "unknown"),
        "failure_class": if status == "pass" {
            "none"
        } else {
            "observability_production_proof_failure"
        },
        "why_failed": failure_summary.unwrap_or_else(|| "none".to_string()),
        "where_failed": if status == "pass" {
            "none"
        } else {
            "observe.snapshot.production_proof"
        },
        "next_repair": next_repair(status, target_event, &query_evidence, &explain_evidence),
        "claim_impact": if status == "pass" {
            "supports_source_local_observability_command_production_proof_only_no_readiness_release_completion_update_goal"
        } else {
            "blocks_observability_command_production_proof_readiness_release_completion_update_goal"
        },
        "supported_claims": if status == "pass" {
            json!(["source_local_observability_command_production_proof"])
        } else {
            json!([])
        },
        "blocked_claims": [
            "final_packet_correctness",
            "review_readiness",
            "package_readiness",
            "release_readiness",
            "completion",
            "update_goal_eligibility",
            "registry_exposure",
            "reviewer_exposure"
        ],
        "receipt_path": command.receipt_rel().to_string_lossy().to_string(),
        "snapshot_command_observability": snapshot_observability,
        "target_event": target_event.cloned().unwrap_or(Value::Null),
        "query_evidence": query_evidence,
        "explain_evidence": explain_evidence,
        "same_candidate_query_proof_paths": query_paths,
        "proof_contract": {
            "requires_target_command_receipt": true,
            "requires_same_candidate_logs_query": true,
            "requires_same_candidate_metrics_query": true,
            "requires_same_candidate_traces_query": true,
            "requires_non_fallback_explain": true,
            "local_spool_only_can_close_gate_92": false
        }
    })
}

fn validate_query_evidence(query_evidence: &Value, failures: &mut Vec<String>) {
    for kind in ["logs", "metrics", "traces"] {
        let row = query_evidence.get(kind).unwrap_or(&Value::Null);
        if row.get("status").and_then(Value::as_str) != Some("pass") {
            failures.push(format!("same_candidate_{kind}_query_not_proven"));
        }
    }
}

fn query_proof_paths(query_evidence: &Value, explain_evidence: &Value) -> Value {
    let mut paths = ["logs", "metrics", "traces"]
        .into_iter()
        .filter_map(|kind| {
            query_evidence
                .get(kind)
                .and_then(|row| row.get("path"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .collect::<Vec<_>>();
    if let Some(path) = explain_evidence.get("path").and_then(Value::as_str) {
        paths.push(path.to_string());
    }
    json!(paths)
}

fn failure_summary(failures: &[String]) -> Option<String> {
    failures.first().map(|first| {
        format!(
            "observability production proof incomplete: total_failures={} first_failure={first}",
            failures.len()
        )
    })
}

fn next_repair(
    status: &str,
    target: Option<&Value>,
    query_evidence: &Value,
    explain_evidence: &Value,
) -> &'static str {
    if status == "pass" {
        return "keep production proof same-candidate and rerun source audit once before any broader claim";
    }
    if target.is_none() {
        return "run the target command once on the current candidate, then rerun observe snapshot with the target run_id";
    }
    if ["logs", "metrics", "traces"].into_iter().any(|kind| {
        query_evidence
            .get(kind)
            .and_then(|row| row.get("status"))
            .and_then(Value::as_str)
            != Some("pass")
    }) {
        return "query logs metrics and traces for the target run/correlation/current digest, then rerun observe snapshot";
    }
    if explain_evidence.get("status").and_then(Value::as_str) != Some("pass") {
        return "run observe explain-failure for the target run and current digest, then rerun observe snapshot";
    }
    "repair the target observability receipt so failure_class why_failed where_failed and next_repair are specific"
}

fn target_missing_failure(selectors: &[(&'static str, &str)]) -> String {
    if selectors.is_empty() {
        return "target_selector_missing".to_string();
    }
    let target = selectors
        .iter()
        .copied()
        .into_iter()
        .map(|(field, value)| format!("{field}={value}"))
        .collect::<Vec<_>>()
        .join(",");
    format!("target_observability_receipt_missing:{target}")
}
