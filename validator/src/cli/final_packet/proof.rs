use serde_json::{Value, json};
use std::path::Path;

pub(super) fn attach_observability(
    root: &Path,
    receipt_path: &Path,
    value: &mut Value,
) -> Result<(), String> {
    attach(root, receipt_path, value, true)
}

pub(super) fn attach_for_evaluation(
    root: &Path,
    receipt_path: &Path,
    value: &mut Value,
) -> Result<(), String> {
    attach(root, receipt_path, value, false)
}

fn attach(root: &Path, receipt_path: &Path, value: &mut Value, emit: bool) -> Result<(), String> {
    let why = why_failed(value);
    let is_pass = status(value) == "pass";
    let claim_impact = if is_pass {
        "final_packet_evidence_dereferenced_only"
    } else {
        "final_packet_correctness_review_readiness_release_completion_update_goal_blocked"
    };
    let next = next_repair(value);
    let failure_class = failure_class(&why);
    let where_failed = where_failed(is_pass);
    let receipt_rel = receipt_display(root, receipt_path);
    let observability = crate::cli::observe::telemetry::command_receipt(
        root,
        crate::cli::observe::telemetry::CommandTelemetry {
            command: "ultragoal final-packet",
            subcommand: "prove",
            operation: "final-packet.prove",
            surface: "source",
            artifact_path: "validation_artifacts/review",
            receipt_path: &receipt_rel,
            status: status(value),
            failure_class: &failure_class,
            why_failed: &why,
            where_failed: &where_failed,
            next_repair: &next,
            claim_impact,
            blocked_claims: string_array(&value["blocked_claim_classes"]),
            supported_claims: supported_claims(value),
            emit,
        },
    )?;
    value["candidate_digest"] = observability["candidate_digest"].clone();
    value["run_id"] = observability["run_id"].clone();
    value["correlation_id"] = observability["correlation_id"].clone();
    value["proof_law_id"] = json!("final-packet-proof");
    value["proof_check_id"] = json!(failure_class);
    value["proof_claim_id"] = json!("final_packet_correctness");
    value["why_failed"] = json!(why);
    value["where_failed"] = json!(where_failed);
    value["next_repair"] = json!(next);
    value["claim_impact"] = json!(claim_impact);
    value["observability"] = observability;
    Ok(())
}

pub(super) fn print_receipt(path: &Path, value: &Value) {
    println!(
        "ultragoal-final-packet {} operation=final-packet.prove candidate={} receipt={} run_id={} correlation_id={} claim_impact={} supported_claims={} unsupported_claims={}",
        status(value),
        value
            .get("candidate_digest")
            .and_then(Value::as_str)
            .unwrap_or("<missing>"),
        path.display(),
        value
            .get("run_id")
            .and_then(Value::as_str)
            .unwrap_or("<missing>"),
        value
            .get("correlation_id")
            .and_then(Value::as_str)
            .unwrap_or("<missing>"),
        value
            .get("claim_impact")
            .and_then(Value::as_str)
            .unwrap_or("<missing>"),
        csv(&value["observability"]["supported_claims"]),
        csv(&value["observability"]["blocked_claims"])
    );
    if status(value) != "pass" {
        print_failure(path, value);
    }
}

fn print_failure(path: &Path, value: &Value) {
    let run_id = value
        .get("run_id")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    println!(
        "failed_check={} why={} where={} claim_impact={} next_repair={} receipt={} run_id={} correlation_id={} query_logs='ultragoal observe logs query --run-id {} --limit 100' query_metrics='ultragoal observe metrics query --run-id {} --limit 100' query_traces='ultragoal observe traces query --run-id {} --limit 100'",
        value
            .get("proof_check_id")
            .and_then(Value::as_str)
            .unwrap_or("final-packet-proof"),
        value
            .get("why_failed")
            .and_then(Value::as_str)
            .unwrap_or("final packet proof failed"),
        value
            .get("where_failed")
            .and_then(Value::as_str)
            .unwrap_or("final-packet.prove"),
        value
            .get("claim_impact")
            .and_then(Value::as_str)
            .unwrap_or("readiness_release_completion_update_goal_blocked"),
        value
            .get("next_repair")
            .and_then(Value::as_str)
            .unwrap_or("inspect final-packet proof references"),
        path.display(),
        run_id,
        value
            .get("correlation_id")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        run_id,
        run_id,
        run_id
    );
}

pub(super) fn status(value: &Value) -> &str {
    value
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("fail")
}

fn why_failed(value: &Value) -> String {
    if status(value) == "pass" {
        return "none".to_string();
    }
    value
        .pointer("/failure/observed_failures")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(Value::as_str)
        .unwrap_or("final_packet_proof_not_proven")
        .to_string()
}

fn next_repair(value: &Value) -> String {
    let why = why_failed(value);
    if why.contains("source_audit") {
        "regenerate or repair the source audit receipt for the current candidate digest".to_string()
    } else if why.contains("registry") {
        "produce live same-surface registry proof or keep registry-dependent claims blocked"
            .to_string()
    } else if why.contains("coverage") {
        "rerun exact coverage and bind the receipt to the current candidate digest".to_string()
    } else if why.contains("packet_absent") {
        "build a current final packet only after source-local proof graph dependencies pass"
            .to_string()
    } else {
        format!("repair final-packet proof dependency: {why}")
    }
}

fn failure_class(why: &str) -> String {
    if why == "none" {
        "none".to_string()
    } else if why.contains("digest_mismatch") || why.contains("_stale") {
        "wrong_digest".to_string()
    } else if why.contains("_missing") || why.contains("packet_absent") {
        "missing_receipt".to_string()
    } else if why.contains("registry") {
        "unsupported_live_surface".to_string()
    } else {
        "claim_blocked".to_string()
    }
}

fn where_failed(is_pass: bool) -> String {
    if is_pass {
        "none".to_string()
    } else {
        "validation_artifacts/review/final-packet-proof.json#/failure/observed_failures/0"
            .to_string()
    }
}

fn receipt_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

fn supported_claims(value: &Value) -> Vec<String> {
    if status(value) == "pass" {
        vec!["final_packet_evidence_dereferenced".to_string()]
    } else {
        vec![]
    }
}

fn string_array(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}

fn csv(value: &Value) -> String {
    string_array(value).join(",")
}
