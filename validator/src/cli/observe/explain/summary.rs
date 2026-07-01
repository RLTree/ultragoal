use serde_json::{Value, json};
use std::path::Path;

pub(super) struct ExplainContext<'a> {
    pub(super) candidate: &'a str,
    pub(super) target_requested: Option<&'a str>,
    pub(super) observed: Option<&'a Value>,
    pub(super) stale_observed: Option<&'a str>,
    pub(super) missing_observed: Option<&'a str>,
    pub(super) opaque_observed: Option<&'a str>,
    pub(super) known_current_failure: &'a Value,
    pub(super) repair_guidance: &'a str,
}

pub(super) fn repair_guidance(
    observed: Option<&Value>,
    stale_observed: Option<&str>,
    missing_observed: Option<&str>,
    opaque_observed: Option<&str>,
) -> String {
    if let Some(failure) = stale_observed {
        return format!(
            "rerun the target command on the current candidate before claiming observability fit: {failure}"
        );
    }
    if let Some(failure) = missing_observed {
        return format!(
            "run the target command once on the current candidate, then query logs metrics and traces before explaining it: {failure}"
        );
    }
    if let Some(failure) = opaque_observed {
        return format!(
            "repair the target command stdout and telemetry envelope so failure_class why_failed where_failed and next_repair are specific: {failure}"
        );
    }
    if observed.is_some_and(|event| event.get("status").and_then(Value::as_str) == Some("pass")) {
        return "No repair for requested telemetry target; keep claim ceiling source-local and run the next gated proof.".to_string();
    }
    observed
        .and_then(|event| event.get("next_repair"))
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty() && *text != "none")
        .unwrap_or(
            "Inspect final-packet proof/source-audit digest dereference, regenerate same-candidate lower-level receipts, and rerun source audit once after implementation changes.",
        )
        .to_string()
}

pub(super) fn explanation(root: &Path, audit_digest: String, ctx: ExplainContext<'_>) -> Value {
    let fallback_used = ctx.observed.is_none();
    let query_evidence = super::query_evidence::for_target(root, ctx.observed, ctx.candidate);
    json!({
        "requested_target": ctx.target_requested,
        "current_source_audit_receipt": "validation_artifacts/ultragoal-audit/validator-receipt.json",
        "current_source_audit_digest": audit_digest,
        "known_current_failure": ctx.known_current_failure,
        "root_cause": root_cause(&ctx),
        "evidence_sources": evidence_sources(ctx.observed, &query_evidence, fallback_used),
        "implicated_paths": implicated_paths(ctx.observed),
        "smallest_repair": ctx.repair_guidance,
        "narrow_rerun": narrow_rerun(ctx.observed),
        "broad_rerun": "run source audit once after the narrow rerun passes if this claim boundary requires broad proof",
        "claim_ceiling": "source-local only; readiness release completion and update_goal remain blocked",
        "fallback_used": fallback_used,
        "query_evidence": query_evidence,
        "repair_guidance": ctx.repair_guidance
    })
}

pub(super) fn target_summary(event: &Value, candidate: &str) -> Value {
    let same_candidate = event.get("candidate_digest").and_then(Value::as_str) == Some(candidate);
    json!({
        "run_id": event.get("run_id").cloned().unwrap_or(Value::Null),
        "correlation_id": event.get("correlation_id").cloned().unwrap_or(Value::Null),
        "candidate_digest": event.get("candidate_digest").cloned().unwrap_or(Value::Null),
        "same_candidate": same_candidate,
        "status": event.get("status").cloned().unwrap_or(Value::Null),
        "failure_class": event.get("failure_class").cloned().unwrap_or(Value::Null),
        "why_failed": event.get("why_failed").cloned().unwrap_or(Value::Null),
        "where_failed": event.get("where_failed").cloned().unwrap_or(Value::Null),
        "next_repair": event.get("next_repair").cloned().unwrap_or(Value::Null),
        "claim_impact": event.get("claim_impact").cloned().unwrap_or(Value::Null),
        "law_id": event.get("law_id").cloned().unwrap_or(Value::Null),
        "check_id": event.get("check_id").cloned().unwrap_or(Value::Null),
        "claim_id": event.get("claim_id").cloned().unwrap_or(Value::Null),
        "artifact_path": event.get("artifact_path").cloned().unwrap_or(Value::Null),
        "receipt_path": event.get("receipt_path").cloned().unwrap_or(Value::Null),
        "duration_ms": event.get("duration_ms").cloned().unwrap_or(Value::Null),
        "worker_count": event.get("worker_count").cloned().unwrap_or(Value::Null),
        "task_count": event.get("task_count").cloned().unwrap_or(Value::Null),
        "queue_depth": event.get("queue_depth").cloned().unwrap_or(Value::Null),
        "cache_mode": event.get("cache_mode").cloned().unwrap_or(Value::Null)
    })
}

fn root_cause(ctx: &ExplainContext<'_>) -> String {
    ctx.stale_observed
        .or(ctx.missing_observed)
        .or(ctx.opaque_observed)
        .map(str::to_string)
        .or_else(|| {
            ctx.observed
                .and_then(|event| event.get("failure_class"))
                .and_then(Value::as_str)
                .filter(|text| *text != "none")
                .map(str::to_string)
        })
        .or_else(|| {
            ctx.known_current_failure
                .as_array()
                .and_then(|items| items.first())
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| "no current failure identified".to_string())
}

fn evidence_sources(
    observed: Option<&Value>,
    query_evidence: &Value,
    fallback_used: bool,
) -> Value {
    let mut sources = Vec::new();
    if observed.is_some() {
        sources.push(json!("target telemetry event"));
        sources.push(json!("target observability receipt"));
    }
    for key in ["logs", "metrics", "traces"] {
        if query_evidence
            .get(key)
            .and_then(|value| value.get("status"))
            .and_then(Value::as_str)
            == Some("pass")
        {
            sources.push(json!(format!("{key} query receipt")));
        }
    }
    if fallback_used {
        sources.push(json!("fallback source-audit/final-packet receipt"));
    }
    Value::Array(sources)
}

fn implicated_paths(observed: Option<&Value>) -> Value {
    let mut paths = Vec::new();
    if let Some(event) = observed {
        for key in ["artifact_path", "receipt_path"] {
            if let Some(path) = event.get(key).and_then(Value::as_str) {
                if !path.is_empty() && path != "none" {
                    paths.push(json!(path));
                }
            }
        }
    }
    Value::Array(paths)
}

fn narrow_rerun(observed: Option<&Value>) -> String {
    let Some(event) = observed else {
        return "run the requested target command once, then rerun explain".to_string();
    };
    let command = event
        .get("command")
        .and_then(Value::as_str)
        .unwrap_or("ultragoal");
    let subcommand = event
        .get("subcommand")
        .and_then(Value::as_str)
        .unwrap_or("");
    format!("{command} {subcommand}")
}
