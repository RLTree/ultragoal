use super::super::nodes::status::measurement_state;
use super::super::nodes::timing::NodeTiming;
use super::super::surfaces::LoopValidationSurface;
use super::claim_evaluation;
use super::measurement_failure;
use super::timing_projection_fields;
use super::verified_local_cache_key;
use serde_json::{Value, json};
use std::time::Instant;

pub(crate) fn surface_record(
    surface: LoopValidationSurface,
    input_digest: &str,
    tier: &str,
    cache_mode: &str,
    baseline_ms: Option<u64>,
    node_timing: Option<NodeTiming>,
) -> Value {
    let started = Instant::now();
    let graph_duration_ms = u64::try_from(started.elapsed().as_millis())
        .unwrap_or(u64::MAX)
        .max(1);
    let cache = cache_decision(
        surface.id,
        input_digest,
        tier,
        cache_mode,
        node_timing.as_ref(),
    );
    let duration_ms = node_timing
        .as_ref()
        .map(|timing| timing.product_latency_ms)
        .unwrap_or(graph_duration_ms);
    let baseline_ms = node_timing
        .as_ref()
        .map(|timing| timing.baseline_duration_ms)
        .or(baseline_ms);
    let measurement = match node_timing.as_ref() {
        Some(timing) if !timing_claim_ready(surface, timing) => {
            measurement_failure::failed_timing_measurement_state(surface, timing)
        }
        Some(_) => measurement_state(surface, duration_ms, baseline_ms),
        None if surface.high_frequency => measurement_state(surface, duration_ms, None),
        None => measurement_state(surface, duration_ms, baseline_ms),
    };
    let mut record = json!({
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
        "hot_loop_policy": surface.hot_loop_policy,
        "duration_ms": duration_ms,
        "graph_evaluation_duration_ms": graph_duration_ms,
        "command": surface.command,
        "canonical_full_command": surface.canonical_full_command,
        "narrow_rerun": surface.narrow_rerun,
        "baseline_measurement_state": measurement.baseline_state,
        "speedup_measurement_state": measurement.speedup_state,
        "baseline_duration_ms": measurement.baseline_duration_ms,
        "verified_local_duration_ms": node_timing.as_ref().map(|timing| timing.verified_local_duration_ms).unwrap_or(duration_ms),
        "speedup_ratio": measurement.speedup_ratio,
        "required_speedup": "20x",
        "claim_impact": measurement.claim_impact
    });
    let object = record
        .as_object_mut()
        .expect("live-loop node projection is always an object");
    timing_projection_fields::insert(object, surface, node_timing.as_ref(), graph_duration_ms);
    claim_evaluation::insert(object, surface, node_timing.as_ref(), &measurement);
    record
}

fn timing_claim_ready(surface: LoopValidationSurface, timing: &NodeTiming) -> bool {
    let timing_row_supports_surface_observation = timing.failure_class == "none"
        && timing.telemetry_reconciliation_status == "pass"
        && timing.validation_status == "pass"
        && timing.observability_status == "pass";
    if !timing_row_supports_surface_observation {
        return false;
    }
    if surface.high_frequency {
        return timing.timing_status == "pass" && timing.speed_claim_status == "supported";
    }
    context_or_boundary_observation_ready(timing)
}

fn context_or_boundary_observation_ready(timing: &NodeTiming) -> bool {
    (timing.timing_status == "partial" && timing.speed_claim_status == "withheld")
        || (timing.timing_status == "pass" && timing.speed_claim_status == "supported")
}

fn cache_decision(
    id: &str,
    input_digest: &str,
    tier: &str,
    cache_mode: &str,
    node_timing: Option<&NodeTiming>,
) -> Value {
    let key = verified_local_cache_key(id, input_digest, tier, cache_mode);
    let cache_enabled = cache_mode == "verified-local";
    let reused = cache_enabled
        && node_timing.is_some_and(|timing| {
            timing.validation_cache_status == "reusable"
                && timing.result_digest == timing.verified_local_result_digest
                && timing.output_digest == timing.verified_local_output_digest
        });
    json!({
        "mode": cache_mode,
        "key": key,
        "hit": reused,
        "invalidation_reason": if reused {
            "verified_current_input_node_timing_row_reused"
        } else if !cache_enabled {
            "cache_disabled_by_requested_cache_mode"
        } else {
            "no verified local cache entry"
        },
        "cache_class": "verified_content_addressed_local",
        "honesty": super::super::context::verify_cache_hit(&key, &key)
    })
}
