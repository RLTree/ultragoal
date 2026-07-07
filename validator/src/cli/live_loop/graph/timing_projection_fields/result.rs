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
        "equivalence_status".to_string(),
        json!(defaults::text(surface, node_timing, |timing| &timing.equivalence_status)),
    );
    object.insert(
        "invalidation_proof".to_string(),
        json!(defaults::text(surface, node_timing, |timing| &timing.invalidation_proof)),
    );
    object.insert(
        "telemetry_reconciliation_status".to_string(),
        json!(defaults::text(surface, node_timing, |timing| &timing
            .telemetry_reconciliation_status)),
    );
    object.insert(
        "validation_status".to_string(),
        json!(defaults::text(surface, node_timing, |timing| &timing.validation_status)),
    );
    object.insert(
        "validation_cache_status".to_string(),
        json!(defaults::text(surface, node_timing, |timing| &timing
            .validation_cache_status)),
    );
    object.insert(
        "observability_status".to_string(),
        json!(defaults::text(surface, node_timing, |timing| &timing.observability_status)),
    );
    object.insert(
        "speed_claim_status".to_string(),
        json!(defaults::text(surface, node_timing, |timing| &timing.speed_claim_status)),
    );
    object.insert(
        "observability_failure_class".to_string(),
        json!(defaults::text(surface, node_timing, |timing| &timing
            .observability_failure_class)),
    );
    insert_reconciliation_fields(object, surface, node_timing);
    insert_digest_fields(object, surface, node_timing);
}

fn insert_reconciliation_fields(
    object: &mut Map<String, Value>,
    surface: LoopValidationSurface,
    node_timing: Option<&NodeTiming>,
) {
    let telemetry = node_timing.map(|timing| &timing.telemetry_reconciliation);
    object.insert(
        "telemetry_reconciliation".to_string(),
        telemetry
            .map(|record| record.value())
            .unwrap_or_else(|| json!({"status": defaults::projection_text(surface)})),
    );
    object.insert(
        "roundtrip_durations_ms".to_string(),
        telemetry
            .and_then(|record| record.field_value("roundtrip_durations_ms"))
            .unwrap_or(Value::Null),
    );
    object.insert(
        "first_failed_roundtrip".to_string(),
        telemetry
            .and_then(|record| record.field_value("first_failed_roundtrip"))
            .unwrap_or(Value::Null),
    );
}

fn insert_digest_fields(
    object: &mut Map<String, Value>,
    surface: LoopValidationSurface,
    node_timing: Option<&NodeTiming>,
) {
    object.insert(
        "verified_local_command".to_string(),
        json!(defaults::text(surface, node_timing, |timing| &timing
            .verified_local_command)),
    );
    object.insert(
        "result_digest".to_string(),
        json!(defaults::text(surface, node_timing, |timing| &timing.result_digest)),
    );
    object.insert(
        "output_digest".to_string(),
        json!(defaults::text(surface, node_timing, |timing| &timing.output_digest)),
    );
    object.insert(
        "verified_local_result_digest".to_string(),
        json!(defaults::text(surface, node_timing, |timing| &timing
            .verified_local_result_digest)),
    );
    object.insert(
        "verified_local_output_digest".to_string(),
        json!(defaults::text(surface, node_timing, |timing| &timing
            .verified_local_output_digest)),
    );
}
