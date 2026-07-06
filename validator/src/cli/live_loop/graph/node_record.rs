use super::super::nodes::status::measurement_state;
use super::super::nodes::timing::NodeTiming;
use super::super::surfaces::LoopValidationSurface;
use super::measurement_failure;
use super::verified_local_cache_key;
use serde_json::{Map, Value, json};
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
    let measurement = match node_timing.as_ref() {
        Some(timing) if !timing_claim_ready(timing) => {
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
        "required_speedup": "20x",
        "claim_impact": measurement.claim_impact
    });
    let object = record
        .as_object_mut()
        .expect("live-loop node projection is always an object");
    insert_timing_fields(object, node_timing.as_ref(), graph_duration_ms);
    record
}

fn timing_claim_ready(timing: &NodeTiming) -> bool {
    timing.timing_status == "pass"
        && timing.failure_class == "none"
        && timing.telemetry_reconciliation_status == "pass"
}

fn insert_timing_fields(
    object: &mut Map<String, Value>,
    node_timing: Option<&NodeTiming>,
    graph_ms: u64,
) {
    object.insert(
        "proof_kind".to_string(),
        json!(text(node_timing, |t| &t.proof_kind)),
    );
    object.insert(
        "cache_hit".to_string(),
        json!(node_timing.map(|timing| timing.cache_hit).unwrap_or(false)),
    );
    object.insert(
        "cache_key".to_string(),
        json!(text(node_timing, |t| &t.cache_key)),
    );
    object.insert(
        "work_unit_count".to_string(),
        json!(
            node_timing
                .map(|timing| timing.work_unit_count)
                .unwrap_or(0)
        ),
    );
    object.insert(
        "actual_work_duration_ms".to_string(),
        json!(
            node_timing
                .map(|timing| timing.actual_work_duration_ms)
                .unwrap_or(0)
        ),
    );
    object.insert(
        "graph_overhead_ms".to_string(),
        json!(
            node_timing
                .map(|timing| timing.graph_overhead_ms)
                .unwrap_or(graph_ms)
        ),
    );
    object.insert(
        "equivalence_status".to_string(),
        json!(text(node_timing, |t| &t.equivalence_status)),
    );
    object.insert(
        "invalidation_proof".to_string(),
        json!(text(node_timing, |t| &t.invalidation_proof)),
    );
    object.insert(
        "telemetry_reconciliation_status".to_string(),
        json!(text(node_timing, |t| &t.telemetry_reconciliation_status)),
    );
    object.insert(
        "verified_local_command".to_string(),
        json!(text(node_timing, |t| &t.verified_local_command)),
    );
    object.insert(
        "result_digest".to_string(),
        json!(text(node_timing, |t| &t.result_digest)),
    );
    object.insert(
        "output_digest".to_string(),
        json!(text(node_timing, |t| &t.output_digest)),
    );
    object.insert(
        "verified_local_result_digest".to_string(),
        json!(text(node_timing, |t| &t.verified_local_result_digest)),
    );
    object.insert(
        "verified_local_output_digest".to_string(),
        json!(text(node_timing, |t| &t.verified_local_output_digest)),
    );
    insert_baseline_fields(object, node_timing);
}

fn insert_baseline_fields(
    object: &mut serde_json::Map<String, Value>,
    node_timing: Option<&NodeTiming>,
) {
    object.insert(
        "baseline_exit_code".to_string(),
        json!(node_timing.and_then(|timing| timing.baseline_exit_code)),
    );
    object.insert(
        "baseline_launch_error".to_string(),
        json!(
            node_timing
                .map(|timing| timing.baseline_launch_error)
                .unwrap_or(false)
        ),
    );
    object.insert(
        "baseline_failed_law".to_string(),
        json!(node_timing.and_then(|timing| timing.baseline_failure.failed_law.as_deref())),
    );
    object.insert(
        "baseline_failed_check".to_string(),
        json!(node_timing.and_then(|timing| timing.baseline_failure.failed_check.as_deref())),
    );
    object.insert(
        "baseline_receipt".to_string(),
        json!(node_timing.and_then(|timing| timing.baseline_failure.receipt.as_deref())),
    );
    object.insert(
        "baseline_run_id".to_string(),
        json!(node_timing.and_then(|timing| timing.baseline_failure.run_id.as_deref())),
    );
    object.insert(
        "baseline_correlation_id".to_string(),
        json!(node_timing.and_then(|timing| timing.baseline_failure.correlation_id.as_deref())),
    );
    object.insert(
        "affected_set_status".to_string(),
        json!(
            node_timing
                .map(|timing| timing.affected_set_status.as_str())
                .unwrap_or("missing_current_timing_record")
        ),
    );
    object.insert(
        "timing_source".to_string(),
        json!(
            node_timing
                .map(|timing| timing.timing_source.as_str())
                .unwrap_or("none")
        ),
    );
}

fn cache_decision(id: &str, input_digest: &str, tier: &str, cache_mode: &str) -> Value {
    let key = verified_local_cache_key(id, input_digest, tier, cache_mode);
    json!({
        "mode": cache_mode,
        "key": key,
        "hit": false,
        "invalidation_reason": "no verified local cache entry",
        "cache_class": "verified_content_addressed_local",
        "honesty": super::super::context::verify_cache_hit(&key, &key)
    })
}

fn text<'a>(
    timing: Option<&'a NodeTiming>,
    field: impl FnOnce(&'a NodeTiming) -> &'a str,
) -> &'a str {
    timing.map(field).unwrap_or("missing")
}
