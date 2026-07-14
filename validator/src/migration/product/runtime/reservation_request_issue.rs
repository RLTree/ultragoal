impl ReservationRequest {
    fn issue(
        plan: &ProductMigrationPlan,
        authorization: &AuthorizationRecord,
    ) -> Result<Self, ProductMigrationError> {
        let semantic_keys = plan
            .effects()
            .iter()
            .map(|effect| effect.semantic_key().to_owned())
            .collect::<Vec<_>>();
        if semantic_keys.is_empty() || semantic_keys.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(ProductMigrationError::new(
                "migration-product-reservation-set-invalid",
            ));
        }
        let operation_id = digest(
            format!(
                "migration-product-operation-v1|{}|{}|{}",
                authorization.authorization_id,
                plan.plan_sha256(),
                semantic_keys.join(",")
            )
            .as_bytes(),
        );
        let reservation_sha256 = digest(
            format!(
                "migration-product-reservation-v1|{}|{}|{}|{}",
                operation_id,
                authorization.binding_sha256,
                plan.plan_sha256(),
                semantic_keys.join(",")
            )
            .as_bytes(),
        );
        Ok(Self {
            schema_version: "MigrationReservationRequest-v1".to_owned(),
            operation_id,
            authorization: authorization.clone(),
            plan_sha256: plan.plan_sha256().to_owned(),
            semantic_keys,
            reservation_sha256,
        })
    }

    pub(crate) fn operation_id(&self) -> &str {
        &self.operation_id
    }

    pub(crate) fn authorization(&self) -> &AuthorizationRecord {
        &self.authorization
    }

    pub(crate) fn semantic_keys(&self) -> &[String] {
        &self.semantic_keys
    }

    pub(crate) fn reservation_sha256(&self) -> &str {
        &self.reservation_sha256
    }

    pub(crate) fn validate_shape(&self) -> bool {
        self.schema_version == "MigrationReservationRequest-v1"
            && valid_sha256(&self.operation_id)
            && self.authorization.validate_shape()
            && self.authorization.plan_sha256 == self.plan_sha256
            && valid_sha256(&self.plan_sha256)
            && !self.semantic_keys.is_empty()
            && self.semantic_keys.windows(2).all(|pair| pair[0] < pair[1])
            && self
                .semantic_keys
                .iter()
                .all(|value| super::super::safe_reference(value))
            && self.reservation_sha256
                == digest(
                    format!(
                        "migration-product-reservation-v1|{}|{}|{}|{}",
                        self.operation_id,
                        self.authorization.binding_sha256,
                        self.plan_sha256,
                        self.semantic_keys.join(",")
                    )
                    .as_bytes(),
                )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum JournalPhase {
    Reserved,
    EffectIntent,
    RollingBack,
    EffectsApplied,
    TerminalApplied,
    TerminalRolledBack,
    Ambiguous,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompatibilityEffectPermitRecord {
    schema_version: String,
    operation_id: String,
    authorization_id: String,
    plan_sha256: String,
    effect_id: String,
    boundary_binding_sha256: String,
    boundary_observation: CompatibilityBoundaryObservation,
    permit_sha256: String,
}

impl CompatibilityEffectPermitRecord {
    fn issue(
        operation_id: &str,
        authorization_id: &str,
        plan_sha256: &str,
        effect: &PlannedMigrationEffect,
        boundary_binding_sha256: &str,
        boundary_observation: CompatibilityBoundaryObservation,
    ) -> Result<Self, ProductMigrationError> {
        let permit_sha256 = compatibility_effect_permit_digest(
            operation_id,
            authorization_id,
            plan_sha256,
            effect,
            boundary_binding_sha256,
            &boundary_observation,
        );
        let value = Self {
            schema_version: "CompatibilityEffectPermit-v1".to_owned(),
            operation_id: operation_id.to_owned(),
            authorization_id: authorization_id.to_owned(),
            plan_sha256: plan_sha256.to_owned(),
            effect_id: effect.effect_id().to_owned(),
            boundary_binding_sha256: boundary_binding_sha256.to_owned(),
            boundary_observation,
            permit_sha256,
        };
        if !value.validate(effect) {
            return Err(ProductMigrationError::new(
                "migration-product-compatibility-effect-permit-invalid",
            ));
        }
        Ok(value)
    }

    fn validate(&self, effect: &PlannedMigrationEffect) -> bool {
        self.schema_version == "CompatibilityEffectPermit-v1"
            && valid_sha256(&self.operation_id)
            && valid_sha256(&self.authorization_id)
            && valid_sha256(&self.plan_sha256)
            && self.effect_id == effect.effect_id()
            && valid_sha256(&self.boundary_binding_sha256)
            && self.boundary_observation.validate()
            && self.permit_sha256
                == compatibility_effect_permit_digest(
                    &self.operation_id,
                    &self.authorization_id,
                    &self.plan_sha256,
                    effect,
                    &self.boundary_binding_sha256,
                    &self.boundary_observation,
                )
    }
}
