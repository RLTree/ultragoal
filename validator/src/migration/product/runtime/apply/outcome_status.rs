impl ApplyOutcome {
    pub(crate) fn status(&self) -> ApplyOutcomeStatus {
        self.status
    }

    pub(crate) fn operation_id(&self) -> &str {
        &self.operation_id
    }

    pub(crate) fn terminal_proof_sha256(&self) -> Option<&str> {
        self.terminal_proof_sha256.as_deref()
    }

    pub(crate) fn to_canonical_json(&self) -> Result<Vec<u8>, ProductMigrationError> {
        let bytes = serde_json::to_vec(self).map_err(|_| {
            ProductMigrationError::new("migration-product-outcome-serialization-failed")
        })?;
        if bytes.len() > MAX_MACHINE_OUTPUT_BYTES {
            return Err(ProductMigrationError::new(
                "migration-product-outcome-too-large",
            ));
        }
        Ok(bytes)
    }

    fn from_operation(operation: &MigrationOperation, repeat: bool, reason: Option<&str>) -> Self {
        let status = match operation.phase {
            JournalPhase::TerminalApplied if repeat => ApplyOutcomeStatus::AlreadyApplied,
            JournalPhase::TerminalApplied => ApplyOutcomeStatus::Applied,
            JournalPhase::TerminalRolledBack => ApplyOutcomeStatus::RolledBack,
            JournalPhase::Ambiguous => ApplyOutcomeStatus::Ambiguous,
            _ => ApplyOutcomeStatus::Interrupted,
        };
        Self {
            schema_version: "MigrationApplyOutcome-v1".to_owned(),
            status,
            operation_id: operation.operation_id.clone(),
            plan_sha256: operation.plan_sha256.clone(),
            phase: operation.phase,
            applied_effect_ids: operation.applied_effect_ids.clone(),
            terminal_proof_sha256: operation.terminal_proof_sha256.clone(),
            reasons: reason.into_iter().map(ToOwned::to_owned).collect(),
            claim_ceiling:
                "source_local_migration_effects_only_not_root_adoption_or_runtime_completion"
                    .to_owned(),
        }
    }
}

fn capture_plan_boundary_observation_allow_crossed(
    plan: &ProductMigrationPlan,
    authority: &dyn ApplyAuthorizationAuthority,
    prior: Option<&CompatibilityBoundaryObservation>,
) -> Result<Option<CompatibilityBoundaryObservation>, ProductMigrationError> {
    let Some(binding) = plan.compatibility_boundary_binding() else {
        return Ok(None);
    };
    let observation = capture_compatibility_boundary_observation(authority)?;
    let prior = prior.unwrap_or_else(|| binding.initial_observation());
    if !binding.observation_follows_source_history(&observation, prior) {
        return Err(ProductMigrationError::new(
            "migration-product-compatibility-boundary-observation-stale-or-substituted",
        ));
    }
    Ok(Some(observation))
}

fn capture_open_plan_boundary_observation(
    plan: &ProductMigrationPlan,
    authority: &dyn ApplyAuthorizationAuthority,
    prior: Option<&CompatibilityBoundaryObservation>,
) -> Result<Option<CompatibilityBoundaryObservation>, ProductMigrationError> {
    let observation = capture_plan_boundary_observation_allow_crossed(plan, authority, prior)?;
    let Some(observation) = observation else {
        return Ok(None);
    };
    let binding = plan.compatibility_boundary_binding().ok_or_else(|| {
        ProductMigrationError::new("migration-product-compatibility-boundary-binding-missing")
    })?;
    if !binding.all_effects_open_at(plan.effects(), &observation) {
        return Err(ProductMigrationError::new(
            "migration-product-compatibility-boundary-crossed",
        ));
    }
    Ok(Some(observation))
}

fn capture_open_effect_boundary_observation(
    plan: &ProductMigrationPlan,
    operation: &MigrationOperation,
    effect: &PlannedMigrationEffect,
    authority: &dyn ApplyAuthorizationAuthority,
) -> Result<CompatibilityBoundaryObservation, ProductMigrationError> {
    let binding = plan.compatibility_boundary_binding().ok_or_else(|| {
        ProductMigrationError::new("migration-product-compatibility-boundary-binding-missing")
    })?;
    let prior = operation
        .last_boundary_observation
        .as_ref()
        .unwrap_or_else(|| binding.initial_observation());
    let observation = capture_compatibility_boundary_observation(authority)?;
    if !binding.observation_follows_source_history(&observation, prior) {
        return Err(ProductMigrationError::new(
            "migration-product-compatibility-boundary-observation-stale-or-substituted",
        ));
    }
    if !binding.effect_is_open_at(effect, &observation) {
        return Err(ProductMigrationError::new(
            "migration-product-compatibility-boundary-crossed",
        ));
    }
    Ok(observation)
}
