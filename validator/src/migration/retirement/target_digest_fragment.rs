impl RetirementTargetProjection {
    fn digest_fragment(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.schema_version,
            self.target_id,
            self.route_id,
            self.source_id,
            self.canonical_target_id,
            self.source_status,
            self.active_reader_count,
            self.active_writer_count,
            self.public_route_count,
            self.generated_output_count,
            self.observed_invocations,
            self.compatibility_window_complete,
            self.inspection_commitment_sha256,
            self.replacement_summary_sha256,
        )
    }
}

#[derive(Eq, PartialEq)]
struct PlanReplacementLedger {
    by_target_id: BTreeMap<String, ConsumedReplacementEvidence>,
}

impl fmt::Debug for PlanReplacementLedger {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PlanReplacementLedger")
            .field("contents", &"<redacted>")
            .finish()
    }
}

impl PlanReplacementLedger {
    fn new() -> Self {
        Self {
            by_target_id: BTreeMap::new(),
        }
    }

    fn insert(
        &mut self,
        target_id: String,
        evidence: ConsumedReplacementEvidence,
    ) -> Result<(), MigrationError> {
        if self.by_target_id.insert(target_id, evidence).is_some() {
            return Err(MigrationError::new(
                "migration-duplicate-private-replacement-evidence",
            ));
        }
        Ok(())
    }

    fn get(&self, target_id: &str) -> Option<&ConsumedReplacementEvidence> {
        self.by_target_id.get(target_id)
    }
}

/// Inspection-only migration plan. Round-tripping this DTO never reconstructs
/// a `MigrationPlan` and cannot carry private replacement authority.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationPlanProjection {
    schema_version: String,
    plan_sha256: String,
    context_commitment_sha256: String,
    route_count: usize,
    target_count: usize,
    targets: Vec<RetirementTargetProjection>,
    projection_sha256: String,
}

#[derive(Eq, PartialEq)]
pub struct MigrationPlan {
    live_context_id: String,
    candidate_id: String,
    catalog_id: String,
    read_session_id: String,
    inventory_sha256: String,
    plan_sha256: String,
    routes: Vec<CompatibilityRoute>,
    targets: Vec<RetirementTarget>,
    replacement_ledger: PlanReplacementLedger,
}

impl fmt::Debug for MigrationPlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MigrationPlan")
            .field("plan_sha256", &self.plan_sha256)
            .field("target_count", &self.targets.len())
            .field("private_replacement_ledger", &"<redacted>")
            .finish()
    }
}

impl ReplacementLedgerBinding {
    fn issue(
        plan: &MigrationPlan,
        target: &RetirementTarget,
        current: &MigrationInventory,
    ) -> Result<Self, MigrationError> {
        plan.verify_current(current)?;
        let evidence = plan
            .replacement_evidence(target)
            .ok_or_else(|| MigrationError::new("migration-private-replacement-evidence-missing"))?;
        if target.route_id != evidence.route_id
            || target.source_id != evidence.source_id
            || target.canonical_target_id != evidence.canonical_target_id
        {
            return Err(MigrationError::new(
                "migration-replacement-ledger-binding-invalid",
            ));
        }
        let target_sha256 = digest(target.digest_fragment().as_bytes());
        let ledger_binding_sha256 = digest(
            format!(
                "replacement-ledger-plan-target-v1|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
                current.live_context_id,
                current.candidate_id,
                current.catalog_id,
                current.read_session_id,
                current.inventory_sha256,
                plan.plan_sha256,
                target.target_id,
                target_sha256,
                target.route_id,
                target.source_id,
                target.canonical_target_id,
                evidence.digest_fragment(),
            )
            .as_bytes(),
        );
        Ok(Self {
            route_id: target.route_id.clone(),
            source_id: target.source_id.clone(),
            canonical_target_id: target.canonical_target_id.clone(),
            live_context_id: current.live_context_id.clone(),
            candidate_id: current.candidate_id.clone(),
            catalog_id: current.catalog_id.clone(),
            read_session_id: current.read_session_id.clone(),
            inventory_sha256: current.inventory_sha256.clone(),
            plan_sha256: plan.plan_sha256.clone(),
            target_id: target.target_id.clone(),
            target_sha256,
            reviewer_id: evidence.reviewer_id.clone(),
            authority_id: evidence.authority_id.clone(),
            authority_session_id: evidence.authority_session_id.clone(),
            nonce_sha256: evidence.nonce_sha256.clone(),
            issued_at_unix_ms: evidence.issued_at_unix_ms,
            expires_at_unix_ms: evidence.expires_at_unix_ms,
            evidence_binding_sha256: evidence.binding_sha256.clone(),
            evidence_id: evidence.evidence_id.clone(),
            attestation_sha256: evidence.attestation_sha256.clone(),
            consumption_sha256: evidence.consumption_sha256.clone(),
            consumption_binding_sha256: evidence.consumption_binding_sha256.clone(),
            ledger_binding_sha256,
        })
    }
}
