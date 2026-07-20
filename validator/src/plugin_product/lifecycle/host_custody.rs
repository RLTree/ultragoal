use super::execution::recovery_state_after_completed_prefix;
use super::model::{
    LifecycleEffect, LifecycleError, LifecycleIntent, LifecyclePlan, LifecycleState,
};
use super::plan::validate_plan;
use crate::distribution::host_effect::{
    DurableHostLifecycleAdmission, HostEffectCompletion, HostEffectCompletionOutcome,
};
use serde::{Deserialize, Serialize};

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
}

#[derive(Clone, Debug, Eq, PartialEq)]
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
        if self.schema_version != "HarnessPluginHostLifecycleRecord-v1"
            || self.issuance_id == 0
            || self.effects.is_empty()
            || super::plan::record_writes_host_state(&self.effects) != self.writes_host_state
        {
            return Err(LifecycleError::InvalidTransition);
        }
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
}

pub(crate) struct HostLifecycleCustody {
    plan: LifecyclePlan,
    pre_effect_record: HostLifecycleRecord,
    effect_cursor: usize,
}

impl HostLifecycleCustody {
    pub(crate) fn take(plan: LifecyclePlan) -> Result<Self, LifecycleError> {
        validate_plan(&plan)?;
        let issuance_id = plan.authorization_seal.issuance_id()?;
        plan.authorization_seal.transfer_to_host()?;
        Ok(Self {
            pre_effect_record: HostLifecycleRecord {
                schema_version: "HarnessPluginHostLifecycleRecord-v1".to_owned(),
                issuance_id,
                plan_id: plan.plan_id.clone(),
                intent: plan.intent,
                before: plan.before.clone(),
                expected_after: plan.expected_after.clone(),
                authorization_sha256: plan.authorization_sha256.clone(),
                rollback_state: plan.rollback_state.clone(),
                effects: plan.effects.clone(),
                writes_host_state: plan.writes_host_state,
            },
            plan,
            effect_cursor: 0,
        })
    }

    pub(crate) fn begin_effects(
        &mut self,
        admission: DurableHostLifecycleAdmission,
    ) -> Result<HostEffectExecutionBinding, LifecycleError> {
        if self.effect_cursor != 0 || admission.record() != &self.pre_effect_record {
            return Err(LifecycleError::ReplayedPlan);
        }
        self.plan.authorization_seal.consume_transferred()?;
        self.effect_cursor = 1;
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
            } if observed == &self.plan.expected_after
                && completed_effects == &self.plan.effects =>
            {
                self.plan.authorization_seal.finish_apply(None)?;
            }
            HostEffectCompletionOutcome::Ambiguous { observed, .. }
                if exact_recovery_state.as_ref() == Some(observed) =>
            {
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
mod tests {
    use super::*;
    use crate::plugin_product::lifecycle::{
        LifecycleAuthorization, LifecycleEffect, LifecycleEffectAdapter, LifecycleIntent,
        LifecycleRequest, PackageAuthority, Version, apply, plan,
    };

    #[test]
    fn transfer_replays_every_plan_clone_before_effect_execution() {
        let before = state('a');
        let plan = plan(
            &before,
            &LifecycleRequest {
                intent: LifecycleIntent::MonotonicUpdate,
                target: Some(authority('b', "1.0.1")),
                prior_authority: None,
                authorization: LifecycleAuthorization {
                    allow_host_write: true,
                    allow_downgrade: false,
                    expected_installed_sha256: Some(digest('a')),
                },
            },
        )
        .unwrap();
        let replay = plan.clone();
        let custody = HostLifecycleCustody::take(plan).unwrap();
        let mut effects = NeverExecute;

        let record = custody.pre_effect_record();
        assert!(record.validate().is_ok());
        assert_eq!(record.plan_id, custody.plan.plan_id);
        assert_eq!(record.before, before);
        assert_eq!(record.effects, custody.effects());
        assert_eq!(
            apply(&before, &replay, &mut effects),
            Err(LifecycleError::ReplayedPlan)
        );
    }

    struct NeverExecute;

    impl LifecycleEffectAdapter for NeverExecute {
        fn execute(&mut self, _: LifecycleEffect, _: &LifecycleState) -> Result<(), String> {
            panic!("replayed transfer reached effect execution")
        }

        fn restore(&mut self, _: &LifecycleState) -> Result<(), String> {
            panic!("replayed transfer reached recovery")
        }

        fn observe_state(&self) -> Result<LifecycleState, String> {
            panic!("replayed transfer reached observation")
        }
    }

    fn state(seed: char) -> LifecycleState {
        LifecycleState {
            installed: Some(authority(seed, "1.0.0")),
            cache: Some(authority(seed, "1.0.0")),
            generation: 1,
            recovery_required: false,
        }
    }

    fn authority(seed: char, version: &str) -> PackageAuthority {
        PackageAuthority {
            version: Version::parse(version).unwrap(),
            package_sha256: digest(seed),
            inventory_sha256: digest('c'),
            candidate_id: digest('d'),
        }
    }

    fn digest(seed: char) -> String {
        format!("sha256:{}", seed.to_string().repeat(64))
    }
}
