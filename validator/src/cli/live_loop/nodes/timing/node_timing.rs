use super::super::command_failure::CommandFailureSummary;
use serde_json::Value;

#[derive(Clone, Debug)]
pub(crate) struct NodeTiming {
    pub(crate) baseline_duration_ms: u64,
    pub(crate) verified_local_duration_ms: u64,
    pub(crate) proof_kind: String,
    pub(crate) cache_hit: bool,
    pub(crate) cache_key: String,
    pub(crate) work_unit_count: u64,
    pub(crate) actual_work_duration_ms: u64,
    pub(crate) graph_overhead_ms: u64,
    pub(crate) reconciled_command_duration_ms: u64,
    pub(crate) product_latency_ms: u64,
    pub(crate) equivalence_status: String,
    pub(crate) invalidation_proof: String,
    pub(crate) telemetry_reconciliation_status: String,
    pub(crate) validation_status: String,
    pub(crate) validation_cache_status: String,
    pub(crate) observability_status: String,
    pub(crate) speed_claim_status: String,
    pub(crate) observability_failure_class: String,
    pub(crate) verified_local_command: String,
    pub(crate) result_digest: String,
    pub(crate) output_digest: String,
    pub(crate) verified_local_result_digest: String,
    pub(crate) verified_local_output_digest: String,
    pub(crate) where_failed: String,
    pub(crate) why_failed: String,
    pub(crate) next_repair: String,
    pub(crate) timing_status: String,
    pub(crate) failure_class: String,
    pub(crate) baseline_proof_kind: String,
    pub(crate) baseline_invalidation_proof: String,
    pub(crate) baseline_exit_code: Option<i32>,
    pub(crate) baseline_launch_error: bool,
    pub(crate) baseline_failure: CommandFailureSummary,
    pub(crate) telemetry_reconciliation: TelemetryReconciliationRecord,
    pub(crate) affected_set_status: String,
    pub(crate) timing_source: String,
}

#[derive(Clone, Debug)]
pub(crate) struct TelemetryReconciliationRecord {
    canonical_json: String,
}

impl TelemetryReconciliationRecord {
    pub(crate) fn from_value(value: &Value) -> Option<Self> {
        value.get("status")?.as_str()?;
        let canonical_json =
            serde_json::to_string(value).expect("serde_json::Value serialization is infallible");
        Some(Self { canonical_json })
    }

    pub(crate) fn value(&self) -> Value {
        serde_json::from_str(&self.canonical_json)
            .expect("telemetry reconciliation record stores canonical JSON")
    }

    pub(crate) fn field_value(&self, key: &str) -> Option<Value> {
        self.value().get(key).cloned()
    }

    fn missing() -> Self {
        Self {
            canonical_json: "{\"status\":\"missing\"}".to_string(),
        }
    }
}

impl From<Value> for TelemetryReconciliationRecord {
    fn from(value: Value) -> Self {
        Self::from_value(&value).unwrap_or_else(Self::missing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn malformed_reconciliation_record_becomes_missing_not_claim_proof() {
        let record = TelemetryReconciliationRecord::from(json!({"why_failed":"opaque"}));

        assert_eq!(record.field_value("status"), Some(json!("missing")));
        assert_eq!(record.value(), json!({"status":"missing"}));
    }

    #[test]
    fn non_text_reconciliation_status_becomes_missing_not_claim_proof() {
        let record = TelemetryReconciliationRecord::from(json!({"status":404}));

        assert_eq!(record.field_value("status"), Some(json!("missing")));
        assert_eq!(record.value(), json!({"status":"missing"}));
    }
}
