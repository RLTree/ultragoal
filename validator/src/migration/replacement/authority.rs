pub(crate) trait ReplacementEvidenceAuthority {
    fn authority_id(&self) -> &str;
    fn reviewer_id(&self, route_id: &str) -> Option<&str>;
    fn session_id(&self) -> &str;
    fn nonce_sha256(&self, route_id: &str) -> Option<&str>;
    fn issued_at_unix_ms(&self) -> u64;
    fn expires_at_unix_ms(&self) -> u64;
    fn now_unix_ms(&self) -> u64;
    fn current_binding(&self) -> (&str, &str, &str, &str, &str);
    fn old_behavior(&self, route_id: &str) -> Option<(&str, EvidenceVerdict, &str)>;
    fn new_behavior(&self, route_id: &str) -> Option<(&str, EvidenceVerdict, &str)>;
    fn representative_journey(&self, route_id: &str) -> Option<(&str, EvidenceVerdict, &str)>;
    fn false_pass_control_results(
        &self,
        route_id: &str,
    ) -> Option<&BTreeMap<String, (EvidenceVerdict, String)>>;
    fn rollback_execution(&self, route_id: &str) -> Option<(&str, EvidenceVerdict, &str)>;
    fn issue_attestation(
        &mut self,
        route_id: &str,
        binding_sha256: &str,
    ) -> Result<String, MigrationError>;
    fn verify_attestation(
        &self,
        route_id: &str,
        binding_sha256: &str,
        attestation_sha256: &str,
    ) -> bool;
    fn consume_once(
        &mut self,
        route_id: &str,
        binding_sha256: &str,
        evidence_id: &str,
        attestation_sha256: &str,
    ) -> Result<String, MigrationError>;
    fn verify_consumed(
        &self,
        route_id: &str,
        binding_sha256: &str,
        evidence_id: &str,
        attestation_sha256: &str,
        consumption_sha256: &str,
    ) -> bool;
    /// Atomically binds an already-consumed evidence row to its exact plan and
    /// target. Implementations must reject missing, revoked, or rolled-back
    /// consumption and conflicting bindings.
    fn bind_plan_target(
        &mut self,
        binding: &ReplacementLedgerBinding,
    ) -> Result<(), MigrationError>;
    /// One atomic read of persistent current/unrevoked consumption and the
    /// exact plan/target binding.
    fn verify_plan_target_consumed(&mut self, binding: &ReplacementLedgerBinding) -> bool;
    /// Atomically elects one final-reconciliation winner for this ledger row.
    fn claim_final_reconciliation(
        &mut self,
        binding: &ReplacementLedgerBinding,
    ) -> Result<String, MigrationError>;
    /// Revalidates the elected winner and the underlying persistent row.
    fn verify_final_claim(
        &mut self,
        binding: &ReplacementLedgerBinding,
        claim_sha256: &str,
    ) -> bool;
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct ReplacementObservation {
    old_behavior_id: String,
    old_verdict: EvidenceVerdict,
    old_result_sha256: String,
    new_behavior_id: String,
    new_verdict: EvidenceVerdict,
    new_result_sha256: String,
    journey_execution_id: String,
    journey_verdict: EvidenceVerdict,
    journey_result_sha256: String,
    false_pass_control_results: BTreeMap<String, (EvidenceVerdict, String)>,
    rollback_execution_id: String,
    rollback_verdict: EvidenceVerdict,
    rollback_result_sha256: String,
}
