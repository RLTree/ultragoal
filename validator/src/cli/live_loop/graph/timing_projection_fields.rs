use super::super::nodes::timing::NodeTiming;
use serde_json::{Map, Value, json};

pub(super) fn insert(
    object: &mut Map<String, Value>,
    node_timing: Option<&NodeTiming>,
    graph_ms: u64,
) {
    insert_work_fields(object, node_timing, graph_ms);
    insert_result_fields(object, node_timing);
    insert_baseline_fields(object, node_timing);
}

fn insert_work_fields(
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
    insert_product_latency_fields(object, node_timing, graph_ms);
}

fn insert_product_latency_fields(
    object: &mut Map<String, Value>,
    node_timing: Option<&NodeTiming>,
    graph_ms: u64,
) {
    object.insert(
        "telemetry_reconciliation_duration_ms".to_string(),
        json!(node_timing.map(telemetry_duration_ms).unwrap_or(0)),
    );
    object.insert(
        "reconciled_command_duration_ms".to_string(),
        json!(
            node_timing
                .map(|timing| timing.reconciled_command_duration_ms)
                .unwrap_or(graph_ms)
        ),
    );
    object.insert(
        "product_latency_ms".to_string(),
        json!(
            node_timing
                .map(|timing| timing.product_latency_ms)
                .unwrap_or(graph_ms)
        ),
    );
}

fn insert_result_fields(object: &mut Map<String, Value>, node_timing: Option<&NodeTiming>) {
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
}

fn insert_baseline_fields(object: &mut Map<String, Value>, node_timing: Option<&NodeTiming>) {
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

fn telemetry_duration_ms(timing: &NodeTiming) -> u64 {
    timing.reconciled_command_duration_ms.saturating_sub(
        timing
            .actual_work_duration_ms
            .saturating_add(timing.graph_overhead_ms),
    )
}

fn text<'a>(
    timing: Option<&'a NodeTiming>,
    field: impl FnOnce(&'a NodeTiming) -> &'a str,
) -> &'a str {
    timing.map(field).unwrap_or("missing")
}
