use super::model::{
    LifecycleEffect, LifecycleError, LifecycleIntent, LifecyclePlan, LifecycleState,
};
use super::plan::validate_plan;
use crate::distribution::host_effect::HostEffectRecoveryHandoff;
use crate::distribution::host_effect::{
    DurableHostLifecycleAdmission, HostEffectCompletion, HostEffectCompletionOutcome,
};
use crate::distribution::{HostCommand, HostCommandPlan, PackageIdentity};
use serde::{Deserialize, Serialize};

include!("observations.rs");
include!("binding.rs");
include!("record.rs");
include!("recovery.rs");
include!("finalization.rs");
include!("recovery_disposition.rs");
#[cfg(test)]
include!("test_release.rs");

pub(crate) struct HostLifecycleCustody {
    plan: LifecyclePlan,
    command_plan: Option<HostCommandPlan>,
    pre_effect_record: HostLifecycleRecord,
    command_cursor: usize,
    effect_cursor: usize,
    custody_phase: u8,
    recovery_required: bool,
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
                command_cursor: 0,
                effect_cursor: 0,
                custody_phase: 0,
                expected_observations: binding.expected_observations,
            },
            plan,
            command_plan: Some(binding.command_plan),
            command_cursor: 0,
            effect_cursor: 0,
            custody_phase: 0,
            recovery_required: false,
        })
    }

    pub(crate) fn begin_effects(
        &mut self,
        admission: &DurableHostLifecycleAdmission,
    ) -> Result<HostEffectExecutionBinding, LifecycleError> {
        if self.custody_phase != 0 || admission.record() != &self.pre_effect_record {
            return Err(LifecycleError::ReplayedPlan);
        }
        self.plan.authorization_seal.consume_transferred()?;
        self.custody_phase = 1;
        debug_assert_eq!(self.pre_effect_record.custody_phase(), 0);
        debug_assert_eq!(self.pre_effect_record.command_cursor(), 0);
        debug_assert_eq!(self.pre_effect_record.effect_cursor(), 0);
        Ok(HostEffectExecutionBinding {
            record: self.pre_effect_record.clone(),
        })
    }

    pub(crate) fn pre_effect_record(&self) -> &HostLifecycleRecord {
        &self.pre_effect_record
    }

    pub(crate) fn completion_binding(&self) -> HostEffectExecutionBinding {
        HostEffectExecutionBinding {
            record: self.pre_effect_record.clone(),
        }
    }

    pub(crate) fn before(&self) -> &LifecycleState {
        &self.plan.before
    }

    pub(crate) fn intent(&self) -> LifecycleIntent {
        self.plan.intent
    }

    pub(crate) fn plan(&self) -> &LifecyclePlan {
        &self.plan
    }

    pub(crate) fn expected_after(&self) -> &LifecycleState {
        &self.plan.expected_after
    }

    #[cfg(test)]
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

    #[cfg(test)]
    pub(crate) fn is_released(&self) -> bool {
        self.command_plan.is_none()
    }

    pub(crate) fn settle(
        &mut self,
        completion: HostEffectCompletion,
    ) -> Result<(), LifecycleError> {
        if self.custody_phase != 1 || completion.binding() != &self.pre_effect_record {
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
        self.recovery_required = matches!(
            completion.outcome(),
            HostEffectCompletionOutcome::Ambiguous { .. }
        );
        match completion.outcome() {
            HostEffectCompletionOutcome::Settled {
                observed,
                completed_effects,
                observations,
            } if observed == &self.plan.expected_after
                && completed_effects == &self.plan.effects =>
            {
                if completion.effect_cursor() != self.plan.effects.len() {
                    return Err(LifecycleError::InvalidTransition);
                }
                if !observations.matches(self.pre_effect_record.expected_observations()) {
                    return Err(LifecycleError::InvalidTransition);
                }
                self.plan.authorization_seal.finish_apply(None)?;
            }
            HostEffectCompletionOutcome::Ambiguous {
                completed_effects,
                observed,
                observations,
                ..
            } if exact_recovery_state.as_ref() == Some(observed) => {
                if completion.effect_cursor() != completed_effects.len() {
                    return Err(LifecycleError::InvalidTransition);
                }
                if !observations.matches(self.pre_effect_record.expected_observations()) {
                    return Err(LifecycleError::InvalidTransition);
                }
                self.plan.authorization_seal.finish_apply(Some(observed))?;
            }
            _ => return Err(LifecycleError::InvalidTransition),
        }
        let command_cursor = completion.observations().command_cursor();
        let effect_cursor = completion.effect_cursor();
        if command_cursor > self.pre_effect_record.command_count()
            || effect_cursor > self.plan.effects.len()
        {
            return Err(LifecycleError::InvalidTransition);
        }
        self.command_cursor = command_cursor;
        self.effect_cursor = effect_cursor;
        self.custody_phase = 2;
        Ok(())
    }

    pub(crate) fn recover<A: super::model::LifecycleEffectAdapter>(
        &mut self,
        observed: &LifecycleState,
        adapter: &mut A,
    ) -> Result<LifecycleState, LifecycleError> {
        if self.custody_phase != 2 || !self.recovery_required {
            return Err(LifecycleError::RecoveryUnavailable);
        }
        let token = super::execution::recovery_token(&self.plan)?;
        let recovered = super::execution::recover(observed, &token, adapter)?;
        self.custody_phase = 3;
        self.recovery_required = false;
        Ok(recovered)
    }

    pub(crate) fn finalization_token(&self) -> Result<HostLifecycleFinalization, LifecycleError> {
        if !matches!(self.custody_phase, 2 | 3) || self.recovery_required {
            return Err(LifecycleError::RecoveryUnavailable);
        }
        Ok(HostLifecycleFinalization {
            record: self.pre_effect_record.clone(),
        })
    }

    pub(crate) fn recovery_pending(&self) -> bool {
        self.custody_phase == 2 && self.recovery_required
    }

    #[cfg(test)]
    pub(crate) fn recovery_token(&self) -> Result<super::model::RecoveryToken, LifecycleError> {
        if self.custody_phase != 2 || !self.recovery_required {
            return Err(LifecycleError::RecoveryUnavailable);
        }
        super::execution::recovery_token(&self.plan)
    }
}

#[cfg(test)]
mod tests;
