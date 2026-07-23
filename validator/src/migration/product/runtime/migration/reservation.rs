impl MigrationOperation {
    fn reserved(
        request: &ReservationRequest,
        plan: &ProductMigrationPlan,
        latest_boundary_observation: Option<CompatibilityBoundaryObservation>,
    ) -> Result<Self, ProductMigrationError> {
        let mut operation = Self {
            schema_version: "MigrationOperation-v2".to_owned(),
            operation_id: request.operation_id.clone(),
            authorization_id: request.authorization.authorization_id.clone(),
            authorization: request.authorization.clone(),
            reservation_sha256: request.reservation_sha256.clone(),
            plan_sha256: plan.plan_sha256().to_owned(),
            input_binding: plan.input_binding().clone(),
            semantic_keys: request.semantic_keys.clone(),
            effects: plan.effects().to_vec(),
            last_boundary_observation: latest_boundary_observation,
            pending_effect_permit: None,
            phase: JournalPhase::Reserved,
            next_effect_index: 0,
            applied_effect_ids: Vec::new(),
            applied_effect_permit_sha256: Vec::new(),
            rollback_effect_index: None,
            terminal_proof_sha256: None,
            revision: 0,
            journal_sha256: String::new(),
        };
        operation.refresh_digest()?;
        Ok(operation)
    }

    pub(crate) fn operation_id(&self) -> &str {
        &self.operation_id
    }

    pub(crate) fn phase(&self) -> JournalPhase {
        self.phase
    }

    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    pub(crate) fn authorization_id(&self) -> &str {
        &self.authorization_id
    }

    pub(crate) fn semantic_keys(&self) -> &[String] {
        &self.semantic_keys
    }

    pub(crate) fn journal_sha256(&self) -> &str {
        &self.journal_sha256
    }

    pub(crate) fn validate_shape(&self) -> bool {
        if self.schema_version != "MigrationOperation-v2"
            || !valid_sha256(&self.operation_id)
            || !valid_sha256(&self.authorization_id)
            || !self.authorization.validate_shape()
            || self.authorization.authorization_id != self.authorization_id
            || self.authorization.plan_sha256 != self.plan_sha256
            || self.authorization.input_binding != self.input_binding
            || self.authorization.effect_set_sha256 != effect_set_digest(&self.effects)
            || !valid_sha256(&self.reservation_sha256)
            || !valid_sha256(&self.plan_sha256)
            || !self.input_binding.validate()
            || self.semantic_keys.is_empty()
            || self.semantic_keys.windows(2).any(|pair| pair[0] >= pair[1])
            || self.effects.len() != self.semantic_keys.len()
            || self.next_effect_index > self.effects.len()
            || self.applied_effect_ids.len() != self.next_effect_index
            || self.applied_effect_permit_sha256.len() != self.next_effect_index
            || self.effects.iter().any(|effect| effect.validate().is_err())
            || self
                .effects
                .iter()
                .zip(&self.semantic_keys)
                .any(|(effect, key)| effect.semantic_key() != key)
            || self
                .applied_effect_ids
                .iter()
                .enumerate()
                .any(|(index, id)| {
                    self.effects
                        .get(index)
                        .is_none_or(|effect| effect.effect_id() != id)
                })
            || self
                .applied_effect_permit_sha256
                .iter()
                .enumerate()
                .any(|(index, permit)| {
                    let Some(effect) = self.effects.get(index) else {
                        return true;
                    };
                    match effect.disposition() {
                        PlanDisposition::AdoptContext => permit.is_some(),
                        PlanDisposition::AdoptCompatibility => {
                            permit.as_deref().is_none_or(|value| !valid_sha256(value))
                        }
                        PlanDisposition::RetireAuthority => permit.is_some(),
                        PlanDisposition::PendingMigration => true,
                    }
                })
            || match (
                self.authorization
                    .compatibility_boundary_observation
                    .as_ref(),
                self.last_boundary_observation.as_ref(),
            ) {
                (Some(initial), Some(last)) => !last.is_same_source_and_monotonic_after(initial),
                (None, None) => false,
                _ => true,
            }
            || self
                .effects
                .iter()
                .any(|effect| effect.disposition() == PlanDisposition::AdoptCompatibility)
                != self
                    .authorization
                    .compatibility_boundary_binding_sha256
                    .is_some()
            || self.pending_effect_permit.as_ref().is_some_and(|permit| {
                !matches!(
                    self.phase,
                    JournalPhase::EffectIntent | JournalPhase::Ambiguous
                ) || self
                    .effects
                    .get(self.next_effect_index)
                    .is_none_or(|effect| !permit.validate(effect))
                    || permit.operation_id != self.operation_id
                    || permit.authorization_id != self.authorization_id
                    || permit.plan_sha256 != self.plan_sha256
                    || self
                        .authorization
                        .compatibility_boundary_binding_sha256
                        .as_deref()
                        != Some(permit.boundary_binding_sha256.as_str())
                    || self.last_boundary_observation.as_ref() != Some(&permit.boundary_observation)
            })
            || self
                .terminal_proof_sha256
                .as_deref()
                .is_some_and(|value| !valid_sha256(value))
        {
            return false;
        }
        self.phase_shape_valid() && self.journal_sha256 == self.compute_digest().unwrap_or_default()
    }

    fn phase_shape_valid(&self) -> bool {
        match self.phase {
            JournalPhase::Reserved => {
                self.rollback_effect_index.is_none()
                    && self.pending_effect_permit.is_none()
                    && self.terminal_proof_sha256.is_none()
            }
            JournalPhase::EffectIntent => {
                self.next_effect_index < self.effects.len()
                    && self.rollback_effect_index.is_none()
                    && self.terminal_proof_sha256.is_none()
            }
            JournalPhase::RollingBack => {
                !self.applied_effect_ids.is_empty()
                    && self.rollback_effect_index == self.applied_effect_ids.len().checked_sub(1)
                    && self.pending_effect_permit.is_none()
                    && self.terminal_proof_sha256.is_none()
            }
            JournalPhase::EffectsApplied => {
                self.next_effect_index == self.effects.len()
                    && self.applied_effect_ids.len() == self.effects.len()
                    && self.rollback_effect_index.is_none()
                    && self.pending_effect_permit.is_none()
                    && self.terminal_proof_sha256.is_none()
            }
            JournalPhase::TerminalApplied => {
                self.next_effect_index == self.effects.len()
                    && self.applied_effect_ids.len() == self.effects.len()
                    && self.rollback_effect_index.is_none()
                    && self.pending_effect_permit.is_none()
                    && self.terminal_proof_sha256.is_some()
            }
            JournalPhase::TerminalRolledBack => {
                self.next_effect_index == 0
                    && self.applied_effect_ids.is_empty()
                    && self.rollback_effect_index.is_none()
                    && self.pending_effect_permit.is_none()
                    && self.terminal_proof_sha256.is_some()
            }
            JournalPhase::Ambiguous => self.terminal_proof_sha256.is_none(),
        }
    }
}
