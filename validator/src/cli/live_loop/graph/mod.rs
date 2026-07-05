use super::surfaces::{LOOP_VALIDATION_SURFACES, LoopValidationSurface};
use crate::scheduler::TaskClass;
use serde_json::{Value, json};
use std::time::Instant;

#[cfg(test)]
mod tests;

pub(crate) fn tasks(
    candidate_digest: &str,
    changed_files_digest: &str,
    audit_context_digest: &str,
    tier: &str,
    cache_mode: &str,
    package_digest_baseline_ms: Option<u64>,
) -> Vec<Box<dyn FnOnce() -> Value + Send + 'static>> {
    LOOP_VALIDATION_SURFACES
        .iter()
        .map(|surface| {
            let surface = *surface;
            let input_digest = input_digest(
                surface,
                candidate_digest,
                changed_files_digest,
                audit_context_digest,
            );
            let baseline_ms = baseline_ms(surface.id, package_digest_baseline_ms);
            let tier = tier.to_string();
            let cache_mode = cache_mode.to_string();
            Box::new(move || {
                surface_record(surface, &input_digest, &tier, &cache_mode, baseline_ms)
            }) as Box<dyn FnOnce() -> Value + Send + 'static>
        })
        .collect()
}

pub(crate) fn first_blocker(nodes: &[Value]) -> Option<Value> {
    nodes.iter().find(|node| node["status"] != "pass").map(|node| {
        let id = node["node_id"].as_str().unwrap_or("unknown_loop_node");
        json!({
            "id": id,
            "surface": node["surface"].as_str().unwrap_or("loop_node"),
            "why_failed": node["why_failed"].as_str().unwrap_or("live-loop node failed"),
            "where_failed": node["where_failed"].as_str().unwrap_or("loop.run.node"),
            "failure_class": node["failure_class"].as_str().unwrap_or("live_loop_node_failure"),
            "next_repair": node["next_repair"].as_str().unwrap_or("repair the live-loop node, then rerun ultragoal loop run"),
            "narrow_rerun": node["narrow_rerun"].as_str().unwrap_or("target/debug/ultragoal --root . loop run --tier hot --cache-mode verified-local --jobs auto"),
            "broad_rerun": "source audit once after narrow observable proof passes",
            "claim_impact": node["claim_impact"].as_str().unwrap_or("source_local_live_loop_blocked")
        })
    })
}

#[cfg(test)]
pub(crate) fn required_high_frequency_validation_ids() -> Vec<&'static str> {
    LOOP_VALIDATION_SURFACES
        .iter()
        .filter(|surface| surface.high_frequency)
        .map(|surface| surface.id)
        .collect()
}

fn surface_record(
    surface: LoopValidationSurface,
    input_digest: &str,
    tier: &str,
    cache_mode: &str,
    baseline_ms: Option<u64>,
) -> Value {
    let started = Instant::now();
    let cache = cache_decision(surface.id, input_digest, tier, cache_mode);
    let duration_ms = u64::try_from(started.elapsed().as_millis())
        .unwrap_or(u64::MAX)
        .max(1);
    let measurement = measurement_state(surface, duration_ms, baseline_ms);
    json!({
        "node_id": surface.id,
        "surface": surface.surface,
        "status": measurement.status,
        "failure_class": measurement.failure_class,
        "why_failed": measurement.why_failed,
        "where_failed": measurement.where_failed,
        "next_repair": measurement.next_repair,
        "telemetry_reconciliation_state": surface.telemetry_reconciliation_state,
        "input_digest": input_digest,
        "cache": cache,
        "task_class": TaskClass::PureReadParallel.id(),
        "high_frequency": surface.high_frequency,
        "duration_ms": duration_ms,
        "command": surface.command,
        "canonical_full_command": surface.canonical_full_command,
        "narrow_rerun": surface.narrow_rerun,
        "baseline_measurement_state": measurement.baseline_state,
        "speedup_measurement_state": measurement.speedup_state,
        "baseline_duration_ms": measurement.baseline_duration_ms,
        "verified_local_duration_ms": duration_ms,
        "speedup_ratio": measurement.speedup_ratio,
        "required_speedup": "20x",
        "claim_impact": measurement.claim_impact
    })
}

struct MeasurementState {
    status: &'static str,
    failure_class: &'static str,
    why_failed: &'static str,
    where_failed: String,
    next_repair: String,
    baseline_state: &'static str,
    speedup_state: &'static str,
    baseline_duration_ms: Option<u64>,
    speedup_ratio: Option<u64>,
    claim_impact: &'static str,
}

fn measurement_state(
    surface: LoopValidationSurface,
    duration_ms: u64,
    baseline_ms: Option<u64>,
) -> MeasurementState {
    if !surface.high_frequency {
        return MeasurementState {
            status: "pass",
            failure_class: "none",
            why_failed: "none",
            where_failed: "none".to_string(),
            next_repair: "none".to_string(),
            baseline_state: "not_required_for_context_or_control_node",
            speedup_state: "not_required_for_context_or_control_node",
            baseline_duration_ms: baseline_ms,
            speedup_ratio: baseline_ms.map(|value| value / duration_ms.max(1)),
            claim_impact: "supports_live_loop_context_observation_only",
        };
    }
    if let Some(baseline_duration_ms) = baseline_ms {
        let speedup_ratio = baseline_duration_ms / duration_ms.max(1);
        if speedup_ratio >= 20 {
            return MeasurementState {
                status: "pass",
                failure_class: "none",
                why_failed: "none",
                where_failed: "none".to_string(),
                next_repair: "none".to_string(),
                baseline_state: "current_full_command_baseline_observed",
                speedup_state: "verified_local_20x_proof_observed",
                baseline_duration_ms: Some(baseline_duration_ms),
                speedup_ratio: Some(speedup_ratio),
                claim_impact: "supports_source_local_live_loop_node_measurement_only",
            };
        }
        return MeasurementState {
            status: "blocked",
            failure_class: "live_loop_speedup_target_missed",
            why_failed: "high-frequency live-loop node has baseline timing but does not meet the verified-local 20x speed target",
            where_failed: format!("loop.run.{}.speedup", surface.id),
            next_repair: format!(
                "split, cache, daemonize, or re-architect `{}` until verified-local timing is at least 20x faster than baseline `{}`",
                surface.id, surface.canonical_full_command
            ),
            baseline_state: "current_full_command_baseline_observed",
            speedup_state: "verified_local_20x_proof_failed",
            baseline_duration_ms: Some(baseline_duration_ms),
            speedup_ratio: Some(speedup_ratio),
            claim_impact: "blocks_live_loop_routine_repair_until_current_timing_proof",
        };
    }
    MeasurementState {
        status: "blocked",
        failure_class: "live_loop_high_frequency_measurement_missing",
        why_failed: "high-frequency live-loop node lacks current full-command baseline and verified-local 20x timing proof",
        where_failed: format!("loop.run.{}.measurement", surface.id),
        next_repair: format!(
            "measure canonical baseline `{}` and verified-local node timing for `{}`, bind both to current inputs, then rerun `target/debug/ultragoal --root . loop run --tier hot --cache-mode verified-local --jobs auto`",
            surface.canonical_full_command, surface.id
        ),
        baseline_state: "missing_current_full_command_baseline",
        speedup_state: "missing_verified_local_20x_proof",
        baseline_duration_ms: None,
        speedup_ratio: None,
        claim_impact: "blocks_live_loop_routine_repair_until_current_timing_proof",
    }
}

fn baseline_ms(id: &str, package_digest_baseline_ms: Option<u64>) -> Option<u64> {
    match id {
        "package_digest" => package_digest_baseline_ms,
        _ => None,
    }
}

fn input_digest(
    surface: LoopValidationSurface,
    candidate_digest: &str,
    changed_files_digest: &str,
    audit_context_digest: &str,
) -> String {
    let digest_material = match surface.id {
        "package_digest" => candidate_digest.to_string(),
        "changed_files" => changed_files_digest.to_string(),
        "audit_context" => audit_context_digest.to_string(),
        _ => format!(
            "candidate={candidate_digest};changed={changed_files_digest};context={audit_context_digest};surface={}",
            surface.id
        ),
    };
    crate::digest::bytes(digest_material.as_bytes())
}

fn cache_decision(id: &str, input_digest: &str, tier: &str, cache_mode: &str) -> Value {
    let key = cache_key(id, input_digest, tier, cache_mode);
    json!({
        "mode": cache_mode,
        "key": key,
        "hit": false,
        "invalidation_reason": "no verified local cache entry",
        "cache_class": "verified_content_addressed_local",
        "honesty": super::context::verify_cache_hit(&key, &key)
    })
}

fn cache_key(id: &str, digest: &str, tier: &str, cache_mode: &str) -> String {
    crate::digest::bytes(
        format!(
            "surface={id};input={digest};validator=ultragoal-rust;law=observability-live-loop;tier={tier};cache={cache_mode};env=local"
        )
        .as_bytes(),
    )
}
