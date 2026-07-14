impl MigrationOperation {
    fn compute_digest(&self) -> Result<String, ProductMigrationError> {
        let bytes = serde_json::to_vec(&(
            (
                &self.schema_version,
                &self.operation_id,
                &self.authorization_id,
                &self.authorization,
                &self.reservation_sha256,
                &self.plan_sha256,
                &self.input_binding,
                &self.semantic_keys,
                &self.effects,
            ),
            (
                &self.last_boundary_observation,
                &self.pending_effect_permit,
                self.phase,
                self.next_effect_index,
                &self.applied_effect_ids,
                &self.applied_effect_permit_sha256,
                self.rollback_effect_index,
                &self.terminal_proof_sha256,
                self.revision,
            ),
        ))
        .map_err(|_| ProductMigrationError::new("migration-product-journal-invalid"))?;
        if bytes.len() > MAX_MACHINE_OUTPUT_BYTES {
            return Err(ProductMigrationError::new(
                "migration-product-journal-too-large",
            ));
        }
        Ok(digest(&bytes))
    }

    fn refresh_digest(&mut self) -> Result<(), ProductMigrationError> {
        self.journal_sha256 = self.compute_digest()?;
        Ok(())
    }

    fn transition(&self, phase: JournalPhase) -> Result<Self, ProductMigrationError> {
        let mut next = self.clone();
        next.phase = phase;
        if !matches!(phase, JournalPhase::EffectIntent | JournalPhase::Ambiguous) {
            next.pending_effect_permit = None;
        }
        next.revision = next.revision.checked_add(1).ok_or_else(|| {
            ProductMigrationError::new("migration-product-journal-revision-exhausted")
        })?;
        next.refresh_digest()?;
        Ok(next)
    }

    fn authorize_current_effect(
        &self,
        permit: CompatibilityEffectPermitRecord,
    ) -> Result<Self, ProductMigrationError> {
        if self.phase != JournalPhase::EffectIntent {
            return Err(ProductMigrationError::new(
                "migration-product-compatibility-effect-permit-phase-invalid",
            ));
        }
        let effect = self.effects.get(self.next_effect_index).ok_or_else(|| {
            ProductMigrationError::new("migration-product-journal-effect-missing")
        })?;
        if !permit.validate(effect)
            || self
                .last_boundary_observation
                .as_ref()
                .is_some_and(|prior| {
                    !permit
                        .boundary_observation
                        .is_same_source_and_monotonic_after(prior)
                })
        {
            return Err(ProductMigrationError::new(
                "migration-product-compatibility-effect-permit-invalid",
            ));
        }
        let mut next = self.clone();
        next.last_boundary_observation = Some(permit.boundary_observation.clone());
        next.pending_effect_permit = Some(permit);
        next.revision = next.revision.checked_add(1).ok_or_else(|| {
            ProductMigrationError::new("migration-product-journal-revision-exhausted")
        })?;
        next.refresh_digest()?;
        Ok(next)
    }

    fn effect_completed(&self) -> Result<Self, ProductMigrationError> {
        let effect = self.effects.get(self.next_effect_index).ok_or_else(|| {
            ProductMigrationError::new("migration-product-journal-effect-missing")
        })?;
        let mut next = self.clone();
        next.applied_effect_ids.push(effect.effect_id().to_owned());
        next.applied_effect_permit_sha256.push(
            next.pending_effect_permit
                .as_ref()
                .map(|permit| permit.permit_sha256.clone()),
        );
        next.next_effect_index += 1;
        next.phase = JournalPhase::Reserved;
        next.pending_effect_permit = None;
        next.revision = next.revision.checked_add(1).ok_or_else(|| {
            ProductMigrationError::new("migration-product-journal-revision-exhausted")
        })?;
        next.refresh_digest()?;
        Ok(next)
    }

    fn begin_rollback(&self) -> Result<Self, ProductMigrationError> {
        let mut next = self.clone();
        next.pending_effect_permit = None;
        if next.applied_effect_ids.is_empty() {
            next.phase = JournalPhase::TerminalRolledBack;
            next.rollback_effect_index = None;
            next.terminal_proof_sha256 = Some(rollback_terminal_proof(&next.operation_id));
        } else {
            next.phase = JournalPhase::RollingBack;
            next.rollback_effect_index = next.applied_effect_ids.len().checked_sub(1);
        }
        next.revision = next.revision.checked_add(1).ok_or_else(|| {
            ProductMigrationError::new("migration-product-journal-revision-exhausted")
        })?;
        next.refresh_digest()?;
        Ok(next)
    }

    fn current_effect_applied_then_begin_rollback(&self) -> Result<Self, ProductMigrationError> {
        let effect = self.effects.get(self.next_effect_index).ok_or_else(|| {
            ProductMigrationError::new("migration-product-journal-effect-missing")
        })?;
        let mut next = self.clone();
        next.applied_effect_ids.push(effect.effect_id().to_owned());
        next.applied_effect_permit_sha256.push(
            next.pending_effect_permit
                .as_ref()
                .map(|permit| permit.permit_sha256.clone()),
        );
        next.next_effect_index += 1;
        next.phase = JournalPhase::RollingBack;
        next.pending_effect_permit = None;
        next.rollback_effect_index = next.applied_effect_ids.len().checked_sub(1);
        next.revision = next.revision.checked_add(1).ok_or_else(|| {
            ProductMigrationError::new("migration-product-journal-revision-exhausted")
        })?;
        next.refresh_digest()?;
        Ok(next)
    }

    fn rollback_completed(&self) -> Result<Self, ProductMigrationError> {
        let mut next = self.clone();
        if let Some(index) = next.rollback_effect_index {
            if index >= next.applied_effect_ids.len() {
                return Err(ProductMigrationError::new(
                    "migration-product-rollback-journal-invalid",
                ));
            }
            next.applied_effect_ids.remove(index);
            next.applied_effect_permit_sha256.remove(index);
            next.next_effect_index = next.applied_effect_ids.len();
            next.rollback_effect_index = index.checked_sub(1);
        }
        if next.applied_effect_ids.is_empty() {
            next.phase = JournalPhase::TerminalRolledBack;
            next.rollback_effect_index = None;
            next.terminal_proof_sha256 = Some(rollback_terminal_proof(&next.operation_id));
        }
        next.revision = next.revision.checked_add(1).ok_or_else(|| {
            ProductMigrationError::new("migration-product-journal-revision-exhausted")
        })?;
        next.refresh_digest()?;
        Ok(next)
    }

    fn terminal_applied(&self, proof_sha256: String) -> Result<Self, ProductMigrationError> {
        let mut next = self.clone();
        next.phase = JournalPhase::TerminalApplied;
        next.terminal_proof_sha256 = Some(proof_sha256);
        next.revision = next.revision.checked_add(1).ok_or_else(|| {
            ProductMigrationError::new("migration-product-journal-revision-exhausted")
        })?;
        next.refresh_digest()?;
        Ok(next)
    }
}
