use super::super::{full_command::FullCommandRun, observation::TelemetryReconciliation};

pub(crate) struct VerifiedLocalProof {
    pub(crate) proof_kind: &'static str,
    pub(crate) cache_hit: bool,
    pub(crate) cache_key: String,
    pub(crate) graph_overhead_ms: u64,
    pub(crate) actual_work: FullCommandRun,
    pub(crate) work_unit_count: u64,
    pub(crate) equivalence_status: String,
    pub(crate) invalidation_proof: String,
    pub(crate) telemetry_reconciliation_status: String,
    pub(crate) telemetry_reconciliation_duration_ms: u64,
    pub(crate) telemetry_reconciliation: TelemetryReconciliation,
    pub(crate) prior_result_digest: Option<String>,
    pub(crate) replayed_output_digest: Option<String>,
    pub(crate) cache_equivalence_status: Option<String>,
}
