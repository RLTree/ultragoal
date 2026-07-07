use super::super::super::nodes::timing::NodeTiming;
use super::super::super::surfaces::LoopValidationSurface;
use super::defaults;
use serde_json::{Map, Value, json};

pub(super) fn insert(
    object: &mut Map<String, Value>,
    surface: LoopValidationSurface,
    node_timing: Option<&NodeTiming>,
    graph_ms: u64,
) {
    object.insert(
        "proof_kind".to_string(),
        json!(defaults::text(surface, node_timing, |timing| &timing.proof_kind)),
    );
    object.insert(
        "cache_hit".to_string(),
        json!(node_timing.map(|timing| timing.cache_hit).unwrap_or(false)),
    );
    object.insert(
        "cache_key".to_string(),
        json!(defaults::text(surface, node_timing, |timing| &timing.cache_key)),
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

fn telemetry_duration_ms(timing: &NodeTiming) -> u64 {
    timing.reconciled_command_duration_ms.saturating_sub(
        timing
            .actual_work_duration_ms
            .saturating_add(timing.graph_overhead_ms),
    )
}
