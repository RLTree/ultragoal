fn rollback_terminal_proof(operation_id: &str) -> String {
    digest(format!("migration-product-rollback-terminal-v1|{operation_id}").as_bytes())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ReservationResult {
    Created(MigrationOperation),
    Existing(MigrationOperation),
}

/// Root-owned implementations must atomically consume an exact registered
/// authorization and reserve every semantic key in `reserve_once`. Partial
/// reservations, replace-on-conflict behavior, and receipt-only rows are
/// contract violations.
pub(crate) trait DurableMigrationStore {
    fn register_authorization(&self, record: &AuthorizationRecord) -> Result<(), StoreFault>;
    fn reserve_once(
        &self,
        request: &ReservationRequest,
        initial: &MigrationOperation,
    ) -> Result<ReservationResult, StoreFault>;
    fn load_operation(&self, operation_id: &str) -> Result<Option<MigrationOperation>, StoreFault>;
    fn compare_and_swap(
        &self,
        operation_id: &str,
        expected_revision: u64,
        expected_journal_sha256: &str,
        next: &MigrationOperation,
    ) -> Result<MigrationOperation, StoreFault>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EffectFault {
    code: &'static str,
    ambiguous: bool,
}

impl EffectFault {
    pub(crate) const fn rejected(code: &'static str) -> Self {
        Self {
            code,
            ambiguous: false,
        }
    }

    pub(crate) const fn ambiguous(code: &'static str) -> Self {
        Self {
            code,
            ambiguous: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EffectObservation {
    schema_version: String,
    authority: AuthoritySnapshot,
    live_read_session_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    effect_permit_sha256: Option<String>,
    observation_sha256: String,
}

impl EffectObservation {
    pub(crate) fn live(
        authority: AuthoritySnapshot,
        live_read_session_sha256: impl Into<String>,
        effect_permit_sha256: Option<String>,
    ) -> Result<Self, ProductMigrationError> {
        let live_read_session_sha256 = live_read_session_sha256.into();
        if !authority.validate()
            || !valid_sha256(&live_read_session_sha256)
            || effect_permit_sha256
                .as_deref()
                .is_some_and(|value| !valid_sha256(value))
        {
            return Err(ProductMigrationError::new(
                "migration-product-live-observation-invalid",
            ));
        }
        let observation_sha256 = digest(
            format!(
                "migration-product-live-observation-v2|{}|{}|{}",
                authority.semantic_sha256()?,
                live_read_session_sha256,
                effect_permit_sha256.as_deref().unwrap_or("not-applicable"),
            )
            .as_bytes(),
        );
        Ok(Self {
            schema_version: "MigrationLiveEffectObservation-v2".to_owned(),
            authority,
            live_read_session_sha256,
            effect_permit_sha256,
            observation_sha256,
        })
    }

    pub(crate) fn authority(&self) -> &AuthoritySnapshot {
        &self.authority
    }

    pub(crate) fn observation_sha256(&self) -> &str {
        &self.observation_sha256
    }

    fn matches(
        &self,
        expected: &AuthoritySnapshot,
        expected_effect_permit_sha256: Option<&str>,
    ) -> bool {
        let Ok(authority_sha256) = self.authority.semantic_sha256() else {
            return false;
        };
        self.schema_version == "MigrationLiveEffectObservation-v2"
            && valid_sha256(&self.live_read_session_sha256)
            && self.authority == *expected
            && self.effect_permit_sha256.as_deref() == expected_effect_permit_sha256
            && self.observation_sha256
                == digest(
                    format!(
                        "migration-product-live-observation-v2|{}|{}|{}",
                        authority_sha256,
                        self.live_read_session_sha256,
                        self.effect_permit_sha256
                            .as_deref()
                            .unwrap_or("not-applicable"),
                    )
                    .as_bytes(),
                )
    }
}

/// The effect boundary has no path, process, shell, delete, or arbitrary write
/// primitive. Implementations receive only an adopted semantic transition,
/// including the exact compatibility prerequisite commitment and the durable
/// trusted-boundary permit digest when present, and must preserve the bound
/// physical bytes. A compatibility implementation must atomically retain the
/// permit digest with its semantic state transition so recovery can distinguish
/// a pre-boundary completed effect from ambiguous post-boundary state.
pub(crate) trait ConfinedMigrationEffect {
    fn observe(
        &mut self,
        effect: &PlannedMigrationEffect,
    ) -> Result<EffectObservation, EffectFault>;
    fn apply(
        &mut self,
        operation_id: &str,
        effect: &PlannedMigrationEffect,
        compatibility_effect_permit_sha256: Option<&str>,
    ) -> Result<EffectObservation, EffectFault>;
    fn rollback(
        &mut self,
        operation_id: &str,
        effect: &PlannedMigrationEffect,
    ) -> Result<EffectObservation, EffectFault>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ApplyOutcomeStatus {
    Applied,
    AlreadyApplied,
    RolledBack,
    Interrupted,
    Ambiguous,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApplyOutcome {
    schema_version: String,
    status: ApplyOutcomeStatus,
    operation_id: String,
    plan_sha256: String,
    phase: JournalPhase,
    applied_effect_ids: Vec<String>,
    terminal_proof_sha256: Option<String>,
    reasons: Vec<String>,
    claim_ceiling: String,
}
