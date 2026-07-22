#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HostLifecycleRecord {
    schema_version: String,
    issuance_id: u64,
    plan_id: String,
    intent: LifecycleIntent,
    before: LifecycleState,
    expected_after: LifecycleState,
    authorization_sha256: String,
    rollback_state: LifecycleState,
    effects: Vec<LifecycleEffect>,
    writes_host_state: bool,
    package: PackageIdentity,
    command_plan: HostCommandPlanRecord,
    scope_sha256: String,
    host_capability_sha256: String,
    command_cursor: usize,
    effect_cursor: usize,
    custody_phase: u8,
    expected_observations: HostLifecycleExpectedObservations,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct HostEffectExecutionBinding {
    record: HostLifecycleRecord,
}

impl HostEffectExecutionBinding {
    pub(crate) fn record(&self) -> &HostLifecycleRecord {
        &self.record
    }
}

impl HostLifecycleRecord {
    pub(crate) fn validate(&self) -> Result<(), LifecycleError> {
        if self.schema_version != "HarnessPluginHostLifecycleRecord-v2"
            || self.issuance_id == 0
            || self.effects.is_empty()
            || super::plan::record_writes_host_state(&self.effects) != self.writes_host_state
            || self.command_cursor > self.command_plan.commands.len()
            || self.effect_cursor > self.effects.len()
            || self.custody_phase > 2
            || !is_digest(&self.scope_sha256)
            || !is_digest(&self.host_capability_sha256)
            || self.expected_observations.validate().is_err()
            || self.command_plan.validate().is_err()
            || self.command_plan.package != self.package
        {
            return Err(LifecycleError::InvalidTransition);
        }
        self.package
            .validate()
            .map_err(|_| LifecycleError::InvalidTransition)?;
        self.before.validate()?;
        self.expected_after.validate()?;
        self.rollback_state.validate()?;
        super::model::validate_digest(&self.authorization_sha256)?;
        if self.rollback_state != self.before
            || super::plan::plan_digest(
                self.intent,
                &self.before,
                &self.expected_after,
                &self.effects,
                self.writes_host_state,
                &self.authorization_sha256,
            )? != self.plan_id
        {
            return Err(LifecycleError::InvalidTransition);
        }
        Ok(())
    }

    pub(crate) fn permit_join(&self) -> (&str, LifecycleIntent) {
        (&self.plan_id, self.intent)
    }

    pub(crate) fn expected_observations(&self) -> &HostLifecycleExpectedObservations {
        &self.expected_observations
    }

    pub(crate) fn package(&self) -> &PackageIdentity {
        &self.package
    }

    pub(crate) fn command_plan_sha256(&self) -> &str {
        self.command_plan.plan_sha256()
    }

    pub(crate) fn command_count(&self) -> usize {
        self.command_plan.commands.len()
    }

    pub(crate) const fn command_cursor(&self) -> usize {
        self.command_cursor
    }

    pub(crate) const fn custody_phase(&self) -> u8 {
        self.custody_phase
    }

    pub(crate) const fn effect_cursor(&self) -> usize {
        self.effect_cursor
    }
}

use sha2::{Digest, Sha256};
