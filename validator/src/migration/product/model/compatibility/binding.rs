impl CompatibilityBoundaryBinding {
    fn issue(
        initial_observation: CompatibilityBoundaryObservation,
        effects: &[PlannedMigrationEffect],
    ) -> Result<Self, ProductMigrationError> {
        if !initial_observation.validate() {
            return Err(ProductMigrationError::new(
                "migration-product-compatibility-boundary-observation-invalid",
            ));
        }
        let mut effect_policies = Vec::new();
        for effect in effects
            .iter()
            .filter(|effect| effect.disposition == PlanDisposition::AdoptCompatibility)
        {
            let prerequisites = effect.compatibility_prerequisites.as_ref().ok_or_else(|| {
                ProductMigrationError::new(
                    "migration-product-compatibility-boundary-policy-missing",
                )
            })?;
            let prerequisites_sha256 = effect
                .compatibility_prerequisites_sha256
                .as_ref()
                .ok_or_else(|| {
                    ProductMigrationError::new(
                        "migration-product-compatibility-boundary-policy-missing",
                    )
                })?;
            if !prerequisites.boundary().is_open_at(&initial_observation) {
                return Err(ProductMigrationError::new(
                    "migration-product-compatibility-boundary-crossed",
                ));
            }
            effect_policies.push(CompatibilityEffectBoundaryPolicy {
                effect_id: effect.effect_id.clone(),
                route_id: effect.route_id.clone(),
                prerequisites_sha256: prerequisites_sha256.clone(),
                boundary: prerequisites.boundary().clone(),
                boundary_sha256: prerequisites.boundary().semantic_sha256()?,
            });
        }
        if effect_policies.is_empty() {
            return Err(ProductMigrationError::new(
                "migration-product-compatibility-boundary-policy-missing",
            ));
        }
        effect_policies.sort_by(|left, right| left.effect_id.cmp(&right.effect_id));
        let binding_sha256 =
            compatibility_boundary_binding_digest(&initial_observation, &effect_policies)?;
        let value = Self {
            schema_version: "CompatibilityBoundaryBinding-v1".to_owned(),
            initial_observation,
            effect_policies,
            binding_sha256,
        };
        if !value.validate_against(effects)? {
            return Err(ProductMigrationError::new(
                "migration-product-compatibility-boundary-binding-invalid",
            ));
        }
        Ok(value)
    }

    pub(super) fn binding_sha256(&self) -> &str {
        &self.binding_sha256
    }

    pub(super) fn initial_observation(&self) -> &CompatibilityBoundaryObservation {
        &self.initial_observation
    }

    pub(super) fn observation_follows_source_history(
        &self,
        observation: &CompatibilityBoundaryObservation,
        prior: &CompatibilityBoundaryObservation,
    ) -> bool {
        observation.is_same_source_and_monotonic_after(&self.initial_observation)
            && observation.is_same_source_and_monotonic_after(prior)
    }

    pub(super) fn effect_is_open_at(
        &self,
        effect: &PlannedMigrationEffect,
        observation: &CompatibilityBoundaryObservation,
    ) -> bool {
        self.effect_policies
            .iter()
            .find(|policy| policy.effect_id == effect.effect_id)
            .is_some_and(|policy| {
                policy.route_id == effect.route_id
                    && effect.compatibility_prerequisites_sha256.as_deref()
                        == Some(policy.prerequisites_sha256.as_str())
                    && policy.boundary_sha256
                        == policy.boundary.semantic_sha256().unwrap_or_default()
                    && policy.boundary.is_open_at(observation)
            })
    }

    pub(super) fn all_effects_open_at(
        &self,
        effects: &[PlannedMigrationEffect],
        observation: &CompatibilityBoundaryObservation,
    ) -> bool {
        effects
            .iter()
            .filter(|effect| effect.disposition == PlanDisposition::AdoptCompatibility)
            .all(|effect| self.effect_is_open_at(effect, observation))
    }

    fn validate_against(
        &self,
        effects: &[PlannedMigrationEffect],
    ) -> Result<bool, ProductMigrationError> {
        if self.schema_version != "CompatibilityBoundaryBinding-v1"
            || !self.initial_observation.validate()
            || self.effect_policies.is_empty()
            || self
                .effect_policies
                .windows(2)
                .any(|pair| pair[0].effect_id >= pair[1].effect_id)
            || self.binding_sha256
                != compatibility_boundary_binding_digest(
                    &self.initial_observation,
                    &self.effect_policies,
                )?
        {
            return Ok(false);
        }
        let compatibility_effects = effects
            .iter()
            .filter(|effect| effect.disposition == PlanDisposition::AdoptCompatibility)
            .collect::<Vec<_>>();
        if compatibility_effects.len() != self.effect_policies.len() {
            return Ok(false);
        }
        Ok(compatibility_effects.iter().all(|effect| {
            self.effect_policies
                .iter()
                .find(|policy| policy.effect_id == effect.effect_id)
                .is_some_and(|policy| {
                    valid_sha256(&policy.effect_id)
                        && valid_identifier(&policy.route_id)
                        && valid_sha256(&policy.prerequisites_sha256)
                        && valid_sha256(&policy.boundary_sha256)
                        && policy.route_id == effect.route_id
                        && effect.compatibility_prerequisites_sha256.as_deref()
                            == Some(policy.prerequisites_sha256.as_str())
                        && policy.boundary_sha256
                            == policy.boundary.semantic_sha256().unwrap_or_default()
                        && policy.boundary.is_open_at(&self.initial_observation)
                })
        }))
    }
}

fn compatibility_boundary_binding_digest(
    initial_observation: &CompatibilityBoundaryObservation,
    policies: &[CompatibilityEffectBoundaryPolicy],
) -> Result<String, ProductMigrationError> {
    let bytes = serde_json::to_vec(&(
        "CompatibilityBoundaryBinding-v1",
        initial_observation,
        policies,
    ))
    .map_err(|_| {
        ProductMigrationError::new("migration-product-compatibility-boundary-binding-invalid")
    })?;
    if bytes.len() > MAX_MACHINE_OUTPUT_BYTES {
        return Err(ProductMigrationError::new(
            "migration-product-compatibility-boundary-binding-too-large",
        ));
    }
    Ok(digest(&bytes))
}
