use super::super::super::nodes::timing::NodeTiming;
use super::super::super::surfaces::LoopValidationSurface;
use super::defaults;
use serde_json::{Map, Value, json};

pub(super) fn insert(
    object: &mut Map<String, Value>,
    surface: LoopValidationSurface,
    node_timing: Option<&NodeTiming>,
) {
    object.insert(
        "baseline_proof_kind".to_string(),
        json!(defaults::text(surface, node_timing, |timing| &timing.baseline_proof_kind)),
    );
    object.insert(
        "baseline_invalidation_proof".to_string(),
        json!(defaults::text(surface, node_timing, |timing| &timing
            .baseline_invalidation_proof)),
    );
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
    insert_failure_identity_fields(object, node_timing);
    object.insert(
        "affected_set_status".to_string(),
        json!(
            node_timing
                .map(|timing| timing.affected_set_status.as_str())
                .unwrap_or_else(|| defaults::affected_set_status(surface))
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

fn insert_failure_identity_fields(
    object: &mut Map<String, Value>,
    node_timing: Option<&NodeTiming>,
) {
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
}
