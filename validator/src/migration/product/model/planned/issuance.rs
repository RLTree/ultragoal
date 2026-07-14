pub(super) struct PlannedMigrationEffectDefinition {
    pub(super) semantic_key: String,
    pub(super) route_id: String,
    pub(super) canonical_target_id: String,
    pub(super) disposition: PlanDisposition,
    pub(super) before: AuthoritySnapshot,
    pub(super) after: AuthoritySnapshot,
    pub(super) adopted_transition_sha256: String,
    pub(super) behavior_execution_sha256: String,
    pub(super) rollback_execution_sha256: String,
    pub(super) false_pass_control_sha256: BTreeMap<String, String>,
    pub(super) compatibility_prerequisites: Option<CompatibilityPrerequisites>,
}

impl PlannedMigrationEffect {
    pub(super) fn issue(
        definition: PlannedMigrationEffectDefinition,
    ) -> Result<Self, ProductMigrationError> {
        let PlannedMigrationEffectDefinition {
            semantic_key,
            route_id,
            canonical_target_id,
            disposition,
            before,
            after,
            adopted_transition_sha256,
            behavior_execution_sha256,
            rollback_execution_sha256,
            false_pass_control_sha256,
            compatibility_prerequisites,
        } = definition;
        let compatibility_prerequisites_sha256 = compatibility_prerequisites
            .as_ref()
            .map(CompatibilityPrerequisites::semantic_sha256)
            .transpose()?;
        let mut effect = Self {
            effect_id: String::new(),
            semantic_key,
            route_id,
            canonical_target_id,
            disposition,
            before,
            after,
            adopted_transition_sha256,
            behavior_execution_sha256,
            rollback_execution_sha256,
            false_pass_control_sha256,
            compatibility_prerequisites,
            compatibility_prerequisites_sha256,
            physical_bytes_policy: PhysicalBytesPolicy::Preserve,
        };
        effect.effect_id = planned_effect_digest(&effect)?;
        effect.validate()?;
        Ok(effect)
    }

    pub(crate) fn effect_id(&self) -> &str {
        &self.effect_id
    }

    pub(crate) fn semantic_key(&self) -> &str {
        &self.semantic_key
    }

    pub(crate) fn route_id(&self) -> &str {
        &self.route_id
    }

    pub(crate) fn disposition(&self) -> PlanDisposition {
        self.disposition
    }

    pub(crate) fn before(&self) -> &AuthoritySnapshot {
        &self.before
    }

    pub(crate) fn after(&self) -> &AuthoritySnapshot {
        &self.after
    }

    pub(crate) fn compatibility_prerequisites_sha256(&self) -> Option<&str> {
        self.compatibility_prerequisites_sha256.as_deref()
    }

    pub(super) fn validate(&self) -> Result<(), ProductMigrationError> {
        if !valid_sha256(&self.effect_id)
            || !safe_reference(&self.semantic_key)
            || !valid_identifier(&self.route_id)
            || !valid_stable_identifier(&self.canonical_target_id)
            || self.disposition == PlanDisposition::PendingMigration
            || !self.before.validate()
            || !self.after.validate()
            || self.before.stable_id != self.after.stable_id
            || self.before.relative_path != self.after.relative_path
            || self.before.digest_sha256 != self.after.digest_sha256
            || self.physical_bytes_policy != PhysicalBytesPolicy::Preserve
            || !valid_sha256(&self.adopted_transition_sha256)
            || !valid_sha256(&self.behavior_execution_sha256)
            || !valid_sha256(&self.rollback_execution_sha256)
            || self.false_pass_control_sha256.len() != REQUIRED_FALSE_PASS_CONTROLS.len()
            || REQUIRED_FALSE_PASS_CONTROLS.iter().any(|control| {
                self.false_pass_control_sha256
                    .get(*control)
                    .is_none_or(|value| !valid_sha256(value))
            })
            || self.compatibility_prerequisites_sha256
                != self
                    .compatibility_prerequisites
                    .as_ref()
                    .map(CompatibilityPrerequisites::semantic_sha256)
                    .transpose()?
            || self.effect_id != planned_effect_digest(self)?
        {
            return Err(ProductMigrationError::new(
                "migration-product-effect-invalid",
            ));
        }
        if self.disposition == PlanDisposition::RetireAuthority
            && (self.after.status != SurfaceStatus::Retired
                || !self.after.active_readers.is_empty()
                || !self.after.active_writers.is_empty()
                || !self.after.public_routes.is_empty()
                || !self.after.generated_outputs.is_empty()
                || self.compatibility_prerequisites.is_some()
                || self.compatibility_prerequisites_sha256.is_some())
        {
            return Err(ProductMigrationError::new(
                "migration-product-retirement-postcondition-invalid",
            ));
        }
        if self.disposition == PlanDisposition::AdoptCompatibility
            && (self.after.status != SurfaceStatus::ContextOnly
                || !self.after.active_readers.is_empty()
                || !self.after.active_writers.is_empty()
                || self.after.public_routes != [self.route_id.clone()]
                || !self.after.generated_outputs.is_empty()
                || self
                    .compatibility_prerequisites
                    .as_ref()
                    .is_none_or(|prerequisites| {
                        !prerequisites.validate(&self.route_id, &self.canonical_target_id)
                    })
                || self.compatibility_prerequisites_sha256.is_none())
        {
            return Err(ProductMigrationError::new(
                "migration-product-compatibility-postcondition-invalid",
            ));
        }
        Ok(())
    }
}
