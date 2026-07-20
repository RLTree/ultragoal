use super::execution::recovery_state_after_completed_prefix;
use super::model::{
    LifecycleEffect, LifecycleError, LifecycleIntent, LifecyclePlan, LifecycleState,
};
use super::plan::validate_plan;
use crate::distribution::host_effect::{
    DurableHostLifecycleAdmission, HostEffectCompletion, HostEffectCompletionOutcome,
};
use crate::distribution::{HostCommand, HostCommandPlan, PackageIdentity};
use serde::{Deserialize, Serialize};

include!("host_custody_observations.rs");
include!("host_custody_binding.rs");
include!("host_custody_record.rs");

pub(crate) struct HostLifecycleCustody {
    plan: LifecyclePlan,
    command_plan: Option<HostCommandPlan>,
    pre_effect_record: HostLifecycleRecord,
    effect_cursor: usize,
}

impl HostLifecycleCustody {
    pub(crate) fn take(
        plan: LifecyclePlan,
        binding: HostLifecycleBinding,
    ) -> Result<Self, LifecycleError> {
        validate_plan(&plan)?;
        let issuance_id = plan.authorization_seal.issuance_id()?;
        plan.authorization_seal.transfer_to_host()?;
        Ok(Self {
            pre_effect_record: HostLifecycleRecord {
                schema_version: "HarnessPluginHostLifecycleRecord-v2".to_owned(),
                issuance_id,
                plan_id: plan.plan_id.clone(),
                intent: plan.intent,
                before: plan.before.clone(),
                expected_after: plan.expected_after.clone(),
                authorization_sha256: plan.authorization_sha256.clone(),
                rollback_state: plan.rollback_state.clone(),
                effects: plan.effects.clone(),
                writes_host_state: plan.writes_host_state,
                package: binding.package,
                command_plan: HostCommandPlanRecord::from_plan(&binding.command_plan),
                scope_sha256: binding.scope_sha256,
                host_capability_sha256: binding.host_capability_sha256,
                effect_cursor: 0,
                expected_observations: binding.expected_observations,
            },
            plan,
            command_plan: Some(binding.command_plan),
            effect_cursor: 0,
        })
    }

    pub(crate) fn begin_effects(
        &mut self,
        admission: &DurableHostLifecycleAdmission,
    ) -> Result<HostEffectExecutionBinding, LifecycleError> {
        if self.effect_cursor != 0 || admission.record() != &self.pre_effect_record {
            return Err(LifecycleError::ReplayedPlan);
        }
        self.plan.authorization_seal.consume_transferred()?;
        self.effect_cursor = 1;
        debug_assert_eq!(self.pre_effect_record.effect_cursor, 0);
        Ok(HostEffectExecutionBinding {
            record: self.pre_effect_record.clone(),
        })
    }

    pub(crate) fn pre_effect_record(&self) -> &HostLifecycleRecord {
        &self.pre_effect_record
    }

    pub(crate) fn before(&self) -> &LifecycleState {
        &self.plan.before
    }

    pub(crate) fn expected_after(&self) -> &LifecycleState {
        &self.plan.expected_after
    }

    pub(crate) fn effects(&self) -> &[LifecycleEffect] {
        &self.plan.effects
    }

    pub(crate) fn plan_sha256(&self) -> &str {
        self.pre_effect_record.command_plan_sha256()
    }

    #[cfg(not(test))]
    pub(crate) fn take_command_plan(&mut self) -> Result<HostCommandPlan, LifecycleError> {
        self.command_plan.take().ok_or(LifecycleError::ReplayedPlan)
    }

    pub(crate) fn candidate_plan(&self) -> Result<HostCommandPlan, LifecycleError> {
        self.command_plan
            .as_ref()
            .cloned()
            .ok_or(LifecycleError::ReplayedPlan)
    }

    pub(crate) fn is_released(&self) -> bool {
        self.command_plan.is_none()
    }

    pub(crate) fn commit_release(&mut self) -> Result<(), LifecycleError> {
        self.command_plan
            .take()
            .map(|_| ())
            .ok_or(LifecycleError::ReplayedPlan)
    }

    pub(crate) fn settle(
        &mut self,
        completion: HostEffectCompletion,
    ) -> Result<(), LifecycleError> {
        if self.effect_cursor != 1 || completion.binding() != &self.pre_effect_record {
            return Err(LifecycleError::InvalidTransition);
        }
        let exact_recovery_state = match completion.outcome() {
            HostEffectCompletionOutcome::Ambiguous {
                completed_effects, ..
            } => Some(recovery_state_after_completed_prefix(
                &self.plan,
                completed_effects,
            )?),
            HostEffectCompletionOutcome::Settled { .. } => None,
        };
        match completion.outcome() {
            HostEffectCompletionOutcome::Settled {
                observed,
                completed_effects,
                observations,
            } if observed == &self.plan.expected_after
                && completed_effects == &self.plan.effects =>
            {
                if !observations.matches(self.pre_effect_record.expected_observations()) {
                    return Err(LifecycleError::InvalidTransition);
                }
                self.plan.authorization_seal.finish_apply(None)?;
            }
            HostEffectCompletionOutcome::Ambiguous {
                observed,
                observations,
                ..
            } if exact_recovery_state.as_ref() == Some(observed) => {
                if !observations.matches(self.pre_effect_record.expected_observations()) {
                    return Err(LifecycleError::InvalidTransition);
                }
                self.plan.authorization_seal.finish_apply(Some(observed))?;
            }
            _ => return Err(LifecycleError::InvalidTransition),
        }
        self.effect_cursor = 2;
        Ok(())
    }

    pub(crate) fn recovery_token(&self) -> Result<super::model::RecoveryToken, LifecycleError> {
        if self.effect_cursor != 2 {
            return Err(LifecycleError::RecoveryUnavailable);
        }
        super::execution::recovery_token(&self.plan)
    }
}

#[cfg(test)]
#[path = "host_custody_tests.rs"]
mod tests;
