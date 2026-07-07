use super::super::observation::TelemetryReconciliation;
use serde_json::Value;

pub(super) fn reconciliation(row: &Value) -> Option<TelemetryReconciliation> {
    let value = row.get("telemetry_reconciliation")?.clone();
    let status = value.get("status").and_then(Value::as_str)?;
    if status.is_empty() {
        return None;
    }
    Some(TelemetryReconciliation {
        status: status.to_string(),
        duration_ms: 1,
        value: serde_json::json!({
            "status": status,
            "reconciliation_mode": "verified_same_candidate_telemetry_reuse",
            "cached_reconciliation_digest": crate::digest::canonical_json(&value),
            "cached_reconciliation": value,
            "claim_impact": "source_local_live_loop_node_observation_only_not_speed_claim"
        }),
    })
}
