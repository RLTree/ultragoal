use super::super::{digest, valid_identifier, valid_sha256};
pub(crate) use super::model::ApplyAuthorizationAuthority;
use super::model::{
    capture_compatibility_boundary_observation, exact_input_matches, AuthoritySnapshot,
    CompatibilityBoundaryObservation, MigrationInputBinding, MigrationInputSource, PlanDisposition,
    PlannedMigrationEffect, ProductMigrationError, ProductMigrationPlan, MAX_APPLY_TTL_MS,
    MAX_MACHINE_OUTPUT_BYTES,
};
use super::registry::derive_product_plan_from_bound_observation;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AuthorizationRecord {
    schema_version: String,
    authorization_id: String,
    principal_id: String,
    authority_id: String,
    authority_session_id: String,
    nonce_sha256: String,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    input_binding: MigrationInputBinding,
    plan_sha256: String,
    effect_set_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    compatibility_boundary_binding_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    compatibility_boundary_observation: Option<CompatibilityBoundaryObservation>,
    binding_sha256: String,
    seal_sha256: String,
}

impl fmt::Debug for AuthorizationRecord {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthorizationRecord")
            .field("authorization_id", &self.authorization_id)
            .field("contents", &"<sealed>")
            .finish()
    }
}

impl AuthorizationRecord {
    pub(crate) fn authorization_id(&self) -> &str {
        &self.authorization_id
    }

    pub(crate) fn binding_sha256(&self) -> &str {
        &self.binding_sha256
    }

    pub(crate) fn seal_sha256(&self) -> &str {
        &self.seal_sha256
    }

    pub(crate) fn plan_sha256(&self) -> &str {
        &self.plan_sha256
    }

    pub(crate) fn input_binding(&self) -> &MigrationInputBinding {
        &self.input_binding
    }

    pub(crate) fn expires_at_unix_ms(&self) -> u64 {
        self.expires_at_unix_ms
    }

    pub(crate) fn validate_shape(&self) -> bool {
        self.schema_version == "MigrationApplyAuthorizationRecord-v2"
            && valid_sha256(&self.authorization_id)
            && valid_identifier(&self.principal_id)
            && valid_identifier(&self.authority_id)
            && self.principal_id != self.authority_id
            && valid_sha256(&self.authority_session_id)
            && self.authority_session_id != self.input_binding.read_session_id()
            && valid_sha256(&self.nonce_sha256)
            && valid_window(
                self.issued_at_unix_ms,
                self.expires_at_unix_ms,
                self.issued_at_unix_ms,
            )
            && self.input_binding.validate()
            && valid_sha256(&self.plan_sha256)
            && valid_sha256(&self.effect_set_sha256)
            && match (
                self.compatibility_boundary_binding_sha256.as_deref(),
                self.compatibility_boundary_observation.as_ref(),
            ) {
                (Some(binding), Some(observation)) => {
                    valid_sha256(binding) && observation.validate()
                }
                (None, None) => true,
                _ => false,
            }
            && valid_sha256(&self.binding_sha256)
            && valid_sha256(&self.seal_sha256)
            && self.authorization_id
                == digest(
                    format!(
                        "migration-apply-authorization-v2|{}|{}",
                        self.binding_sha256, self.seal_sha256
                    )
                    .as_bytes(),
                )
    }
}

#[derive(Eq, PartialEq)]
pub(crate) struct ApplyAuthorization {
    record: AuthorizationRecord,
}

impl fmt::Debug for ApplyAuthorization {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ApplyAuthorization")
            .field("authorization_id", &self.record.authorization_id)
            .field("contents", &"<sealed-single-use>")
            .finish()
    }
}

impl ApplyAuthorization {
    pub(crate) fn authorization_id(&self) -> &str {
        &self.record.authorization_id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StoreFault {
    code: &'static str,
}

impl StoreFault {
    pub(crate) const fn new(code: &'static str) -> Self {
        Self { code }
    }

    pub(crate) const fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for StoreFault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code)
    }
}

impl std::error::Error for StoreFault {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReservationRequest {
    schema_version: String,
    operation_id: String,
    authorization: AuthorizationRecord,
    plan_sha256: String,
    semantic_keys: Vec<String>,
    reservation_sha256: String,
}

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

fn compatibility_effect_permit_digest(
    operation_id: &str,
    authorization_id: &str,
    plan_sha256: &str,
    effect: &PlannedMigrationEffect,
    boundary_binding_sha256: &str,
    observation: &CompatibilityBoundaryObservation,
) -> String {
    digest(
        format!(
            "migration-compatibility-effect-permit-v1|{}|{}|{}|{}|{}|{}|{}",
            operation_id,
            authorization_id,
            plan_sha256,
            effect.effect_id(),
            effect
                .compatibility_prerequisites_sha256()
                .unwrap_or("missing"),
            boundary_binding_sha256,
            observation.observation_sha256(),
        )
        .as_bytes(),
    )
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MigrationOperation {
    schema_version: String,
    operation_id: String,
    authorization_id: String,
    authorization: AuthorizationRecord,
    reservation_sha256: String,
    plan_sha256: String,
    input_binding: MigrationInputBinding,
    semantic_keys: Vec<String>,
    effects: Vec<PlannedMigrationEffect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_boundary_observation: Option<CompatibilityBoundaryObservation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pending_effect_permit: Option<CompatibilityEffectPermitRecord>,
    phase: JournalPhase,
    next_effect_index: usize,
    applied_effect_ids: Vec<String>,
    applied_effect_permit_sha256: Vec<Option<String>>,
    rollback_effect_index: Option<usize>,
    terminal_proof_sha256: Option<String>,
    revision: u64,
    journal_sha256: String,
}

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

    pub(crate) const fn code(&self) -> &'static str {
        self.code
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
    if !binding.observation_matches_source_and_progress(&observation, prior) {
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
    if !binding.observation_matches_source_and_progress(&observation, prior) {
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

pub(crate) fn issue_apply_authorization(
    plan: &ProductMigrationPlan,
    source: &mut dyn MigrationInputSource,
    authority: &mut dyn ApplyAuthorizationAuthority,
    store: &dyn DurableMigrationStore,
) -> Result<ApplyAuthorization, ProductMigrationError> {
    plan.validate()?;
    if plan.effects().is_empty() {
        return Err(ProductMigrationError::new(
            "migration-product-plan-has-no-adopted-effects",
        ));
    }
    let current = source.capture()?;
    current.validate()?;
    let exact = derive_product_plan_from_bound_observation(
        &current,
        plan.compatibility_boundary_binding()
            .map(|binding| binding.initial_observation()),
    )?;
    if !exact_input_matches(plan.input_binding(), &current)
        || exact.plan_sha256() != plan.plan_sha256()
    {
        return Err(ProductMigrationError::new(
            "migration-product-plan-stale-or-substituted",
        ));
    }
    source.revalidate(plan.input_binding(), &[])?;
    let compatibility_boundary_observation =
        capture_open_plan_boundary_observation(plan, authority, None)?;

    let principal_id = authority.principal_id().to_owned();
    let authority_id = authority.authority_id().to_owned();
    let authority_session_id = authority.session_id().to_owned();
    let nonce_sha256 = authority.nonce_sha256().to_owned();
    let issued_at_unix_ms = authority.issued_at_unix_ms();
    let expires_at_unix_ms = authority.expires_at_unix_ms();
    let (bound_input, bound_plan) = authority.current_binding();
    if !valid_identifier(&principal_id)
        || !valid_identifier(&authority_id)
        || principal_id == authority_id
        || !valid_sha256(&authority_session_id)
        || authority_session_id == plan.input_binding().read_session_id()
        || !valid_sha256(&nonce_sha256)
        || !valid_window(
            issued_at_unix_ms,
            expires_at_unix_ms,
            authority.now_unix_ms(),
        )
        || bound_input != plan.input_binding().binding_sha256()
        || bound_plan != plan.plan_sha256()
    {
        return Err(ProductMigrationError::new(
            "migration-product-authorization-issuer-refused",
        ));
    }
    let effect_set_sha256 = effect_set_digest(plan.effects());
    let binding_sha256 = authorization_binding(
        &principal_id,
        &authority_id,
        &authority_session_id,
        &nonce_sha256,
        issued_at_unix_ms,
        expires_at_unix_ms,
        plan.input_binding(),
        plan.plan_sha256(),
        &effect_set_sha256,
        plan.compatibility_boundary_binding()
            .map(|binding| binding.binding_sha256()),
        compatibility_boundary_observation
            .as_ref()
            .map(CompatibilityBoundaryObservation::observation_sha256),
    );
    let seal_sha256 = authority.seal(&binding_sha256)?;
    let (current_input, current_plan) = authority.current_binding();
    if !valid_sha256(&seal_sha256)
        || !authority.verify_seal(&binding_sha256, &seal_sha256)
        || authority.principal_id() != principal_id
        || authority.authority_id() != authority_id
        || authority.session_id() != authority_session_id
        || authority.nonce_sha256() != nonce_sha256
        || authority.issued_at_unix_ms() != issued_at_unix_ms
        || authority.expires_at_unix_ms() != expires_at_unix_ms
        || current_input != plan.input_binding().binding_sha256()
        || current_plan != plan.plan_sha256()
        || !valid_window(
            issued_at_unix_ms,
            expires_at_unix_ms,
            authority.now_unix_ms(),
        )
    {
        return Err(ProductMigrationError::new(
            "migration-product-authorization-seal-refused",
        ));
    }
    let post_seal_boundary_observation = capture_open_plan_boundary_observation(
        plan,
        authority,
        compatibility_boundary_observation.as_ref(),
    )?;
    if compatibility_boundary_observation.is_some() != post_seal_boundary_observation.is_some() {
        return Err(ProductMigrationError::new(
            "migration-product-compatibility-boundary-authorization-refused",
        ));
    }
    let authorization_id = digest(
        format!(
            "migration-apply-authorization-v2|{}|{}",
            binding_sha256, seal_sha256
        )
        .as_bytes(),
    );
    let record = AuthorizationRecord {
        schema_version: "MigrationApplyAuthorizationRecord-v2".to_owned(),
        authorization_id,
        principal_id,
        authority_id,
        authority_session_id,
        nonce_sha256,
        issued_at_unix_ms,
        expires_at_unix_ms,
        input_binding: plan.input_binding().clone(),
        plan_sha256: plan.plan_sha256().to_owned(),
        effect_set_sha256,
        compatibility_boundary_binding_sha256: plan
            .compatibility_boundary_binding()
            .map(|binding| binding.binding_sha256().to_owned()),
        compatibility_boundary_observation,
        binding_sha256,
        seal_sha256,
    };
    if !record.validate_shape() {
        return Err(ProductMigrationError::new(
            "migration-product-authorization-invalid",
        ));
    }
    store
        .register_authorization(&record)
        .map_err(|_| ProductMigrationError::new("migration-product-authorization-store-refused"))?;
    Ok(ApplyAuthorization { record })
}

pub(crate) fn apply_product_plan(
    plan: &ProductMigrationPlan,
    authorization: &ApplyAuthorization,
    source: &mut dyn MigrationInputSource,
    authority: &dyn ApplyAuthorizationAuthority,
    store: &dyn DurableMigrationStore,
    effects: &mut dyn ConfinedMigrationEffect,
) -> Result<ApplyOutcome, ProductMigrationError> {
    plan.validate()?;
    validate_authorization(plan, &authorization.record, authority)?;
    let current = source.capture()?;
    let exact = derive_product_plan_from_bound_observation(
        &current,
        plan.compatibility_boundary_binding()
            .map(|binding| binding.initial_observation()),
    )?;
    if !exact_input_matches(plan.input_binding(), &current)
        || exact.plan_sha256() != plan.plan_sha256()
    {
        return Err(ProductMigrationError::new(
            "migration-product-apply-input-stale",
        ));
    }
    source.revalidate(plan.input_binding(), &[])?;
    let latest_boundary_observation = capture_open_plan_boundary_observation(
        plan,
        authority,
        authorization
            .record
            .compatibility_boundary_observation
            .as_ref(),
    )?;
    let request = ReservationRequest::issue(plan, &authorization.record)?;
    let initial = MigrationOperation::reserved(&request, plan, latest_boundary_observation)?;
    let reservation = store
        .reserve_once(&request, &initial)
        .map_err(|_| ProductMigrationError::new("migration-product-reservation-refused"))?;
    let (operation, repeat) = match reservation {
        ReservationResult::Created(operation) if operation == initial => (operation, false),
        ReservationResult::Created(_) => {
            return Err(ProductMigrationError::new(
                "migration-product-reservation-substituted",
            ));
        }
        ReservationResult::Existing(operation) => (operation, true),
    };
    verify_operation(&operation, plan, &request)?;
    if !operation_boundary_seals_valid(&operation, plan, authority) {
        return Err(ProductMigrationError::new(
            "migration-product-reservation-substituted",
        ));
    }
    drive_operation(operation, plan, source, authority, store, effects, repeat)
}

pub(crate) fn recover_product_operation(
    operation_id: &str,
    plan: &ProductMigrationPlan,
    source: &mut dyn MigrationInputSource,
    boundary_authority: &dyn ApplyAuthorizationAuthority,
    store: &dyn DurableMigrationStore,
    effects: &mut dyn ConfinedMigrationEffect,
) -> Result<ApplyOutcome, ProductMigrationError> {
    if !valid_sha256(operation_id) {
        return Err(ProductMigrationError::new(
            "migration-product-recovery-id-invalid",
        ));
    }
    plan.validate()?;
    let operation = store
        .load_operation(operation_id)
        .map_err(|_| ProductMigrationError::new("migration-product-recovery-store-failed"))?
        .ok_or_else(|| ProductMigrationError::new("migration-product-recovery-unknown"))?;
    if !operation.validate_shape()
        || operation.operation_id != operation_id
        || operation.plan_sha256 != plan.plan_sha256()
        || operation.input_binding != *plan.input_binding()
        || operation.effects != plan.effects()
        || operation.authorization.plan_sha256 != plan.plan_sha256()
        || operation.authorization.effect_set_sha256 != effect_set_digest(plan.effects())
        || operation
            .authorization
            .compatibility_boundary_binding_sha256
            .as_deref()
            != plan
                .compatibility_boundary_binding()
                .map(|binding| binding.binding_sha256())
        || !operation_boundary_seals_valid(&operation, plan, boundary_authority)
        || boundary_authority.principal_id() != operation.authorization.principal_id
        || boundary_authority.authority_id() != operation.authorization.authority_id
        || boundary_authority.session_id() != operation.authorization.authority_session_id
        || boundary_authority.nonce_sha256() != operation.authorization.nonce_sha256
        || !boundary_authority.verify_seal(
            &operation.authorization.binding_sha256,
            &operation.authorization.seal_sha256,
        )
    {
        return Err(ProductMigrationError::new(
            "migration-product-recovery-substituted",
        ));
    }
    capture_plan_boundary_observation_allow_crossed(
        plan,
        boundary_authority,
        operation.last_boundary_observation.as_ref(),
    )?;
    drive_operation(
        operation,
        plan,
        source,
        boundary_authority,
        store,
        effects,
        true,
    )
}

fn drive_operation(
    mut operation: MigrationOperation,
    plan: &ProductMigrationPlan,
    source: &mut dyn MigrationInputSource,
    boundary_authority: &dyn ApplyAuthorizationAuthority,
    store: &dyn DurableMigrationStore,
    effects: &mut dyn ConfinedMigrationEffect,
    repeat: bool,
) -> Result<ApplyOutcome, ProductMigrationError> {
    for _ in 0..(plan.effects().len().saturating_mul(6).saturating_add(12)) {
        if !operation.validate_shape() {
            return Err(ProductMigrationError::new(
                "migration-product-journal-mutated",
            ));
        }
        match operation.phase {
            JournalPhase::TerminalApplied
            | JournalPhase::TerminalRolledBack
            | JournalPhase::Ambiguous => {
                return Ok(ApplyOutcome::from_operation(&operation, repeat, None));
            }
            JournalPhase::Reserved => {
                if source
                    .revalidate(&operation.input_binding, &operation.applied_effect_ids)
                    .is_err()
                {
                    let ambiguous = operation.transition(JournalPhase::Ambiguous)?;
                    operation = cas_or_interrupted(store, &operation, &ambiguous)?;
                    continue;
                }
                if operation.next_effect_index == operation.effects.len() {
                    let next = operation.transition(JournalPhase::EffectsApplied)?;
                    operation = cas_or_interrupted(store, &operation, &next)?;
                    continue;
                }
                let effect = &operation.effects[operation.next_effect_index];
                match effects.observe(effect) {
                    Ok(observation) if observation.matches(effect.before(), None) => {
                        let next = operation.transition(JournalPhase::EffectIntent)?;
                        operation = cas_or_interrupted(store, &operation, &next)?;
                    }
                    Ok(_) | Err(_) => {
                        let ambiguous = operation.transition(JournalPhase::Ambiguous)?;
                        operation = cas_or_interrupted(store, &operation, &ambiguous)?;
                    }
                }
            }
            JournalPhase::EffectIntent => {
                let effect = operation.effects[operation.next_effect_index].clone();
                let pending_permit_sha256 = operation
                    .pending_effect_permit
                    .as_ref()
                    .map(|permit| permit.permit_sha256.as_str());
                match effects.observe(&effect) {
                    Ok(observation)
                        if observation.matches(effect.after(), pending_permit_sha256)
                            && (effect.disposition() != PlanDisposition::AdoptCompatibility
                                || pending_permit_sha256.is_some()) =>
                    {
                        let next = operation.effect_completed()?;
                        operation = cas_or_interrupted(store, &operation, &next)?;
                    }
                    Ok(observation) if observation.matches(effect.before(), None) => {
                        if effect.disposition() == PlanDisposition::AdoptCompatibility {
                            let observation = match capture_open_effect_boundary_observation(
                                plan,
                                &operation,
                                &effect,
                                boundary_authority,
                            ) {
                                Ok(observation) => observation,
                                Err(error)
                                    if error.code()
                                        == "migration-product-compatibility-boundary-crossed"
                                        && !operation.applied_effect_ids.is_empty() =>
                                {
                                    let rollback = operation.begin_rollback()?;
                                    operation = cas_or_interrupted(store, &operation, &rollback)?;
                                    continue;
                                }
                                Err(error) => return Err(error),
                            };
                            let binding =
                                plan.compatibility_boundary_binding().ok_or_else(|| {
                                    ProductMigrationError::new(
                                        "migration-product-compatibility-boundary-binding-missing",
                                    )
                                })?;
                            let permit = CompatibilityEffectPermitRecord::issue(
                                &operation.operation_id,
                                &operation.authorization_id,
                                &operation.plan_sha256,
                                &effect,
                                binding.binding_sha256(),
                                observation,
                            )?;
                            let authorized = operation.authorize_current_effect(permit)?;
                            operation = cas_or_interrupted(store, &operation, &authorized)?;
                        }
                        let permit_sha256 = operation
                            .pending_effect_permit
                            .as_ref()
                            .map(|permit| permit.permit_sha256.as_str());
                        match effects.apply(&operation.operation_id, &effect, permit_sha256) {
                            Ok(_) => match effects.observe(&effect) {
                                Ok(confirmed)
                                    if confirmed.matches(effect.after(), permit_sha256) =>
                                {
                                    let next = operation.effect_completed()?;
                                    operation = cas_or_interrupted(store, &operation, &next)?;
                                }
                                Ok(confirmed) if confirmed.matches(effect.before(), None) => {
                                    let rollback = operation.begin_rollback()?;
                                    operation = cas_or_interrupted(store, &operation, &rollback)?;
                                }
                                _ => {
                                    let ambiguous =
                                        operation.transition(JournalPhase::Ambiguous)?;
                                    operation = cas_or_interrupted(store, &operation, &ambiguous)?;
                                }
                            },
                            Err(fault) if !fault.ambiguous => match effects.observe(&effect) {
                                Ok(confirmed) if confirmed.matches(effect.before(), None) => {
                                    let rollback = operation.begin_rollback()?;
                                    operation = cas_or_interrupted(store, &operation, &rollback)?;
                                }
                                Ok(confirmed)
                                    if confirmed.matches(effect.after(), permit_sha256) =>
                                {
                                    let rollback =
                                        operation.current_effect_applied_then_begin_rollback()?;
                                    operation = cas_or_interrupted(store, &operation, &rollback)?;
                                }
                                _ => {
                                    let ambiguous =
                                        operation.transition(JournalPhase::Ambiguous)?;
                                    operation = cas_or_interrupted(store, &operation, &ambiguous)?;
                                }
                            },
                            Err(_) => {
                                let ambiguous = operation.transition(JournalPhase::Ambiguous)?;
                                operation = cas_or_interrupted(store, &operation, &ambiguous)?;
                            }
                        }
                    }
                    _ => {
                        let ambiguous = operation.transition(JournalPhase::Ambiguous)?;
                        operation = cas_or_interrupted(store, &operation, &ambiguous)?;
                    }
                }
            }
            JournalPhase::RollingBack => {
                let Some(index) = operation.rollback_effect_index else {
                    return Err(ProductMigrationError::new(
                        "migration-product-rollback-journal-invalid",
                    ));
                };
                let effect = operation.effects[index].clone();
                let applied_permit_sha256 = operation
                    .applied_effect_permit_sha256
                    .get(index)
                    .and_then(|permit| permit.as_deref());
                match effects.observe(&effect) {
                    Ok(observation) if observation.matches(effect.before(), None) => {
                        let next = operation.rollback_completed()?;
                        operation = cas_or_interrupted(store, &operation, &next)?;
                    }
                    Ok(observation)
                        if observation.matches(effect.after(), applied_permit_sha256) =>
                    {
                        match effects.rollback(&operation.operation_id, &effect) {
                            Ok(rolled_back) if rolled_back.matches(effect.before(), None) => {
                                match effects.observe(&effect) {
                                    Ok(confirmed) if confirmed.matches(effect.before(), None) => {
                                        let next = operation.rollback_completed()?;
                                        operation = cas_or_interrupted(store, &operation, &next)?;
                                    }
                                    _ => {
                                        let ambiguous =
                                            operation.transition(JournalPhase::Ambiguous)?;
                                        operation =
                                            cas_or_interrupted(store, &operation, &ambiguous)?;
                                    }
                                }
                            }
                            _ => {
                                let ambiguous = operation.transition(JournalPhase::Ambiguous)?;
                                operation = cas_or_interrupted(store, &operation, &ambiguous)?;
                            }
                        }
                    }
                    _ => {
                        let ambiguous = operation.transition(JournalPhase::Ambiguous)?;
                        operation = cas_or_interrupted(store, &operation, &ambiguous)?;
                    }
                }
            }
            JournalPhase::EffectsApplied => {
                if source
                    .revalidate(&operation.input_binding, &operation.applied_effect_ids)
                    .is_err()
                {
                    let ambiguous = operation.transition(JournalPhase::Ambiguous)?;
                    operation = cas_or_interrupted(store, &operation, &ambiguous)?;
                    continue;
                }
                let mut observations = Vec::with_capacity(operation.effects.len());
                let mut terminal_valid = true;
                for (index, effect) in operation.effects.iter().enumerate() {
                    let applied_permit_sha256 = operation
                        .applied_effect_permit_sha256
                        .get(index)
                        .and_then(|permit| permit.as_deref());
                    match effects.observe(effect) {
                        Ok(observation)
                            if observation.matches(effect.after(), applied_permit_sha256) =>
                        {
                            if effect.disposition() == PlanDisposition::RetireAuthority
                                && (observation.authority().status()
                                    != super::super::SurfaceStatus::Retired
                                    || !observation.authority().active_readers().is_empty()
                                    || !observation.authority().active_writers().is_empty()
                                    || !observation.authority().public_routes().is_empty()
                                    || !observation.authority().generated_outputs().is_empty())
                            {
                                terminal_valid = false;
                                break;
                            }
                            observations.push(observation.observation_sha256().to_owned());
                        }
                        _ => {
                            terminal_valid = false;
                            break;
                        }
                    }
                }
                if !terminal_valid {
                    let ambiguous = operation.transition(JournalPhase::Ambiguous)?;
                    operation = cas_or_interrupted(store, &operation, &ambiguous)?;
                    continue;
                }
                let proof = digest(
                    format!(
                        "migration-product-terminal-semantic-proof-v1|{}|{}|{}",
                        operation.operation_id,
                        operation.plan_sha256,
                        observations.join(",")
                    )
                    .as_bytes(),
                );
                let terminal = operation.terminal_applied(proof)?;
                operation = cas_or_interrupted(store, &operation, &terminal)?;
            }
        }
    }
    Ok(ApplyOutcome::from_operation(
        &operation,
        repeat,
        Some("migration-product-step-bound-exhausted"),
    ))
}

fn cas_or_interrupted(
    store: &dyn DurableMigrationStore,
    current: &MigrationOperation,
    next: &MigrationOperation,
) -> Result<MigrationOperation, ProductMigrationError> {
    match store.compare_and_swap(
        current.operation_id(),
        current.revision(),
        current.journal_sha256(),
        next,
    ) {
        Ok(stored) => {
            if stored != *next || !stored.validate_shape() {
                Err(ProductMigrationError::new(
                    "migration-product-store-substituted-journal",
                ))
            } else {
                Ok(stored)
            }
        }
        Err(_) => Err(ProductMigrationError::new(
            "migration-product-operation-interrupted",
        )),
    }
}

fn verify_operation(
    operation: &MigrationOperation,
    plan: &ProductMigrationPlan,
    request: &ReservationRequest,
) -> Result<(), ProductMigrationError> {
    if !operation.validate_shape()
        || operation.operation_id != request.operation_id
        || operation.authorization_id != request.authorization.authorization_id
        || operation.authorization != request.authorization
        || operation.reservation_sha256 != request.reservation_sha256
        || operation.plan_sha256 != plan.plan_sha256()
        || operation.input_binding != *plan.input_binding()
        || operation.semantic_keys != request.semantic_keys
        || operation.effects != plan.effects()
    {
        return Err(ProductMigrationError::new(
            "migration-product-reservation-substituted",
        ));
    }
    Ok(())
}

fn operation_boundary_seals_valid(
    operation: &MigrationOperation,
    plan: &ProductMigrationPlan,
    authority: &dyn ApplyAuthorizationAuthority,
) -> bool {
    match plan.compatibility_boundary_binding() {
        None => {
            operation
                .authorization
                .compatibility_boundary_observation
                .is_none()
                && operation.last_boundary_observation.is_none()
                && operation.pending_effect_permit.is_none()
        }
        Some(binding) => {
            binding.initial_observation().seal_verified_by(authority)
                && operation
                    .authorization
                    .compatibility_boundary_observation
                    .as_ref()
                    .is_some_and(|observation| observation.seal_verified_by(authority))
                && operation
                    .last_boundary_observation
                    .as_ref()
                    .is_some_and(|observation| observation.seal_verified_by(authority))
                && operation
                    .pending_effect_permit
                    .as_ref()
                    .is_none_or(|permit| permit.boundary_observation.seal_verified_by(authority))
        }
    }
}

fn validate_authorization(
    plan: &ProductMigrationPlan,
    record: &AuthorizationRecord,
    authority: &dyn ApplyAuthorizationAuthority,
) -> Result<(), ProductMigrationError> {
    let expected_binding = authorization_binding(
        &record.principal_id,
        &record.authority_id,
        &record.authority_session_id,
        &record.nonce_sha256,
        record.issued_at_unix_ms,
        record.expires_at_unix_ms,
        plan.input_binding(),
        plan.plan_sha256(),
        &effect_set_digest(plan.effects()),
        record.compatibility_boundary_binding_sha256.as_deref(),
        record
            .compatibility_boundary_observation
            .as_ref()
            .map(CompatibilityBoundaryObservation::observation_sha256),
    );
    let compatibility_boundary_valid = match (
        plan.compatibility_boundary_binding(),
        record.compatibility_boundary_binding_sha256.as_deref(),
        record.compatibility_boundary_observation.as_ref(),
    ) {
        (Some(binding), Some(binding_sha256), Some(observation)) => {
            binding.binding_sha256() == binding_sha256
                && binding.initial_observation().seal_verified_by(authority)
                && observation.seal_verified_by(authority)
                && binding.observation_matches_source_and_progress(
                    observation,
                    binding.initial_observation(),
                )
                && binding.all_effects_open_at(plan.effects(), observation)
        }
        (None, None, None) => true,
        _ => false,
    };
    let (bound_input, bound_plan) = authority.current_binding();
    if !record.validate_shape()
        || record.input_binding != *plan.input_binding()
        || record.plan_sha256 != plan.plan_sha256()
        || record.effect_set_sha256 != effect_set_digest(plan.effects())
        || !compatibility_boundary_valid
        || record.binding_sha256 != expected_binding
        || authority.principal_id() != record.principal_id
        || authority.authority_id() != record.authority_id
        || authority.session_id() != record.authority_session_id
        || authority.nonce_sha256() != record.nonce_sha256
        || authority.issued_at_unix_ms() != record.issued_at_unix_ms
        || authority.expires_at_unix_ms() != record.expires_at_unix_ms
        || bound_input != plan.input_binding().binding_sha256()
        || bound_plan != plan.plan_sha256()
        || !valid_window(
            record.issued_at_unix_ms,
            record.expires_at_unix_ms,
            authority.now_unix_ms(),
        )
        || !authority.verify_seal(&record.binding_sha256, &record.seal_sha256)
    {
        return Err(ProductMigrationError::new(
            "migration-product-authorization-stale-or-rebound",
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn authorization_binding(
    principal_id: &str,
    authority_id: &str,
    session_id: &str,
    nonce_sha256: &str,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    input: &MigrationInputBinding,
    plan_sha256: &str,
    effect_set_sha256: &str,
    compatibility_boundary_binding_sha256: Option<&str>,
    compatibility_boundary_observation_sha256: Option<&str>,
) -> String {
    digest(
        format!(
            "migration-apply-authorization-binding-v2|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            principal_id,
            authority_id,
            session_id,
            nonce_sha256,
            issued_at_unix_ms,
            expires_at_unix_ms,
            input.binding_sha256(),
            plan_sha256,
            effect_set_sha256,
            compatibility_boundary_binding_sha256.unwrap_or("not-applicable"),
            compatibility_boundary_observation_sha256.unwrap_or("not-applicable"),
        )
        .as_bytes(),
    )
}

fn effect_set_digest(effects: &[PlannedMigrationEffect]) -> String {
    digest(
        format!(
            "migration-product-effect-set-v1|{}",
            effects
                .iter()
                .map(|effect| format!(
                    "{}={}@{}",
                    effect.semantic_key(),
                    effect.effect_id(),
                    effect
                        .compatibility_prerequisites_sha256()
                        .unwrap_or("not-applicable")
                ))
                .collect::<Vec<_>>()
                .join(",")
        )
        .as_bytes(),
    )
}

fn valid_window(issued_at: u64, expires_at: u64, now: u64) -> bool {
    issued_at <= now
        && now <= expires_at
        && issued_at < expires_at
        && expires_at.saturating_sub(issued_at) <= MAX_APPLY_TTL_MS
}
