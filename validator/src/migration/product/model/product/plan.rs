impl ProductMigrationPlan {
    pub(super) fn issue(
        input_binding: MigrationInputBinding,
        contract_id: String,
        mut items: Vec<ProductPlanItem>,
        mut effects: Vec<PlannedMigrationEffect>,
        compatibility_boundary_observation: Option<CompatibilityBoundaryObservation>,
    ) -> Result<Self, ProductMigrationError> {
        if items.is_empty() || items.len() > MAX_PRODUCT_ITEMS || effects.len() > items.len() {
            return Err(ProductMigrationError::new(
                "migration-product-plan-count-refused",
            ));
        }
        items.sort_by(|left, right| {
            (&left.route_id, &left.source_id).cmp(&(&right.route_id, &right.source_id))
        });
        effects.sort_by(|left, right| left.semantic_key.cmp(&right.semantic_key));
        if items.windows(2).any(|pair| {
            pair[0].route_id == pair[1].route_id && pair[0].source_id == pair[1].source_id
        }) || effects
            .windows(2)
            .any(|pair| pair[0].semantic_key == pair[1].semantic_key)
        {
            return Err(ProductMigrationError::new(
                "migration-product-plan-duplicate",
            ));
        }
        let has_compatibility_effect = effects
            .iter()
            .any(|effect| effect.disposition == PlanDisposition::AdoptCompatibility);
        let compatibility_boundary_binding =
            match (has_compatibility_effect, compatibility_boundary_observation) {
                (true, Some(observation)) => {
                    Some(CompatibilityBoundaryBinding::issue(observation, &effects)?)
                }
                (true, None) => {
                    return Err(ProductMigrationError::new(
                        "migration-product-compatibility-boundary-authority-required",
                    ));
                }
                (false, None) => None,
                (false, Some(_)) => {
                    return Err(ProductMigrationError::new(
                        "migration-product-compatibility-boundary-unexpected",
                    ));
                }
            };
        let registry_sha256 = input_binding.registry_sha256.clone();
        let plan_sha256 = product_plan_digest(
            &input_binding,
            &contract_id,
            &compatibility_boundary_binding,
            &items,
            &effects,
        )?;
        let boundary_binding_sha256 = compatibility_boundary_binding
            .as_ref()
            .map(CompatibilityBoundaryBinding::binding_sha256)
            .unwrap_or("not-applicable");
        let projection_sha256 = digest(
            format!(
                "migration-product-plan-projection-v2|{}|{}|{}|{}|{}|{}",
                plan_sha256,
                input_binding.binding_sha256,
                contract_id,
                boundary_binding_sha256,
                items.len(),
                effects.len(),
            )
            .as_bytes(),
        );
        let projection = ProductMigrationPlanProjection {
            schema_version: "ProductMigrationPlanProjection-v2".to_owned(),
            input_binding,
            contract_id,
            registry_sha256,
            compatibility_boundary_binding,
            item_count: items.len(),
            effect_count: effects.len(),
            items,
            effects,
            plan_sha256,
            projection_sha256,
        };
        let encoded = serde_json::to_vec(&projection)
            .map_err(|_| ProductMigrationError::new("migration-product-plan-output-invalid"))?;
        if encoded.len() > MAX_MACHINE_OUTPUT_BYTES {
            return Err(ProductMigrationError::new(
                "migration-product-plan-output-too-large",
            ));
        }
        Ok(Self { projection })
    }

    pub(crate) fn plan_sha256(&self) -> &str {
        &self.projection.plan_sha256
    }

    pub(crate) fn input_binding(&self) -> &MigrationInputBinding {
        &self.projection.input_binding
    }

    pub(crate) fn effects(&self) -> &[PlannedMigrationEffect] {
        &self.projection.effects
    }

    pub(super) fn compatibility_boundary_binding(&self) -> Option<&CompatibilityBoundaryBinding> {
        self.projection.compatibility_boundary_binding.as_ref()
    }

    pub(crate) fn items(&self) -> &[ProductPlanItem] {
        &self.projection.items
    }

    pub(crate) fn projection(&self) -> ProductMigrationPlanProjection {
        self.projection.clone()
    }

    pub(crate) fn verify_projection(
        &self,
        projection: &ProductMigrationPlanProjection,
    ) -> Result<(), ProductMigrationError> {
        if &self.projection != projection {
            return Err(ProductMigrationError::new(
                "migration-product-plan-projection-substituted",
            ));
        }
        Ok(())
    }

    pub(super) fn validate(&self) -> Result<(), ProductMigrationError> {
        let expected_boundary_binding =
            match self.projection.compatibility_boundary_binding.as_ref() {
                Some(binding) => {
                    if !binding.validate_against(&self.projection.effects)? {
                        return Err(ProductMigrationError::new("migration-product-plan-mutated"));
                    }
                    binding.binding_sha256()
                }
                None => {
                    if self
                        .projection
                        .effects
                        .iter()
                        .any(|effect| effect.disposition == PlanDisposition::AdoptCompatibility)
                    {
                        return Err(ProductMigrationError::new("migration-product-plan-mutated"));
                    }
                    "not-applicable"
                }
            };
        if self.projection.schema_version != "ProductMigrationPlanProjection-v2"
            || !self.projection.input_binding.validate()
            || self.projection.registry_sha256 != self.projection.input_binding.registry_sha256
            || self.projection.item_count != self.projection.items.len()
            || self.projection.effect_count != self.projection.effects.len()
            || self
                .projection
                .effects
                .iter()
                .any(|effect| effect.validate().is_err())
            || self.projection.plan_sha256
                != product_plan_digest(
                    &self.projection.input_binding,
                    &self.projection.contract_id,
                    &self.projection.compatibility_boundary_binding,
                    &self.projection.items,
                    &self.projection.effects,
                )?
            || self.projection.projection_sha256
                != digest(
                    format!(
                        "migration-product-plan-projection-v2|{}|{}|{}|{}|{}|{}",
                        self.projection.plan_sha256,
                        self.projection.input_binding.binding_sha256,
                        self.projection.contract_id,
                        expected_boundary_binding,
                        self.projection.items.len(),
                        self.projection.effects.len(),
                    )
                    .as_bytes(),
                )
        {
            return Err(ProductMigrationError::new("migration-product-plan-mutated"));
        }
        Ok(())
    }
}
