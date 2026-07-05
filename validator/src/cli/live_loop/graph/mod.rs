use super::nodes::status::measurement_state;
use super::nodes::timing::NodeTiming;
use super::surfaces::{LOOP_VALIDATION_SURFACES, LoopValidationSurface};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::time::Instant;

mod measurement_failure;
#[cfg(test)]
mod tests;

pub(crate) fn tasks(
    candidate_digest: &str,
    changed_files_digest: &str,
    audit_context_digest: &str,
    tier: &str,
    cache_mode: &str,
    package_digest_baseline_ms: Option<u64>,
    node_timings: &BTreeMap<String, NodeTiming>,
) -> Vec<Box<dyn FnOnce() -> Value + Send + 'static>> {
    LOOP_VALIDATION_SURFACES
        .iter()
        .map(|surface| {
            let surface = *surface;
            let input_digest = surface_input_digest(
                surface,
                candidate_digest,
                changed_files_digest,
                audit_context_digest,
            );
            let baseline_ms = baseline_ms(surface.id, package_digest_baseline_ms);
            let node_timing = node_timings.get(surface.id).cloned();
            let tier = tier.to_string();
            let cache_mode = cache_mode.to_string();
            Box::new(move || {
                surface_record(
                    surface,
                    &input_digest,
                    &tier,
                    &cache_mode,
                    baseline_ms,
                    node_timing,
                )
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
    node_timing: Option<NodeTiming>,
) -> Value {
    let started = Instant::now();
    let cache = cache_decision(surface.id, input_digest, tier, cache_mode);
    let graph_duration_ms = u64::try_from(started.elapsed().as_millis())
        .unwrap_or(u64::MAX)
        .max(1);
    let duration_ms = node_timing
        .as_ref()
        .map(|timing| timing.verified_local_duration_ms)
        .unwrap_or(graph_duration_ms);
    let baseline_ms = node_timing
        .as_ref()
        .map(|timing| timing.baseline_duration_ms)
        .or(baseline_ms);
    let measurement = if let Some(timing) = node_timing
        .as_ref()
        .filter(|timing| timing.timing_status != "pass")
    {
        measurement_failure::failed_timing_measurement_state(surface, timing)
    } else {
        measurement_state(surface, duration_ms, baseline_ms)
    };
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
        "graph_task_class": crate::scheduler::TaskClass::PureReadParallel.id(),
        "execution_task_class": surface.execution_task_class.id(),
        "execution_serial_reason": surface.execution_serial_reason,
        "high_frequency": surface.high_frequency,
        "duration_ms": duration_ms,
        "graph_evaluation_duration_ms": graph_duration_ms,
        "command": surface.command,
        "canonical_full_command": surface.canonical_full_command,
        "narrow_rerun": surface.narrow_rerun,
        "baseline_measurement_state": measurement.baseline_state,
        "speedup_measurement_state": measurement.speedup_state,
        "baseline_duration_ms": measurement.baseline_duration_ms,
        "verified_local_duration_ms": duration_ms,
        "speedup_ratio": measurement.speedup_ratio,
        "baseline_exit_code": node_timing
            .as_ref()
            .and_then(|timing| timing.baseline_exit_code),
        "baseline_launch_error": node_timing
            .as_ref()
            .map(|timing| timing.baseline_launch_error)
            .unwrap_or(false),
        "baseline_failed_law": node_timing
            .as_ref()
            .and_then(|timing| timing.baseline_failure.failed_law.as_deref()),
        "baseline_failed_check": node_timing
            .as_ref()
            .and_then(|timing| timing.baseline_failure.failed_check.as_deref()),
        "baseline_receipt": node_timing
            .as_ref()
            .and_then(|timing| timing.baseline_failure.receipt.as_deref()),
        "baseline_run_id": node_timing
            .as_ref()
            .and_then(|timing| timing.baseline_failure.run_id.as_deref()),
        "baseline_correlation_id": node_timing
            .as_ref()
            .and_then(|timing| timing.baseline_failure.correlation_id.as_deref()),
        "required_speedup": "20x",
        "affected_set_status": node_timing
            .as_ref()
            .map(|timing| timing.affected_set_status.as_str())
            .unwrap_or("missing_current_timing_record"),
        "timing_source": node_timing
            .as_ref()
            .map(|timing| timing.timing_source.as_str())
            .unwrap_or("none"),
        "claim_impact": measurement.claim_impact
    })
}

fn baseline_ms(id: &str, package_digest_baseline_ms: Option<u64>) -> Option<u64> {
    match id {
        "package_digest" => package_digest_baseline_ms,
        _ => None,
    }
}

pub(crate) fn surface_input_digest(
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
    let key = verified_local_cache_key(id, input_digest, tier, cache_mode);
    json!({
        "mode": cache_mode,
        "key": key,
        "hit": false,
        "invalidation_reason": "no verified local cache entry",
        "cache_class": "verified_content_addressed_local",
        "honesty": super::context::verify_cache_hit(&key, &key)
    })
}

pub(crate) fn verified_local_cache_key(
    id: &str,
    digest: &str,
    tier: &str,
    cache_mode: &str,
) -> String {
    crate::digest::bytes(
        format!(
            "surface={id};input={digest};validator=ultragoal-rust;law=observability-live-loop;tier={tier};cache={cache_mode};env=local"
        )
        .as_bytes(),
    )
}
