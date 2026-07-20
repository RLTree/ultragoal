use super::model::{
    LifecycleEffect, LifecycleError, LifecycleIntent, LifecyclePlan, LifecycleState,
};
use super::plan::validate_plan;
use serde::Serialize;

/// Immutable data that a host ledger must persist before an effect begins.
/// This record conveys no authority: only `HostLifecycleCustody` can advance
/// the shared plan seal.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HostLifecycleRecord {
    schema_version: &'static str,
    plan_id: String,
    intent: LifecycleIntent,
    before: LifecycleState,
    expected_after: LifecycleState,
    effects: Vec<LifecycleEffect>,
    writes_host_state: bool,
}

/// The sole transferable ownership form of a sealed plugin lifecycle plan.
///
/// Moving a plan here advances its shared seal before any host effect can be
/// prepared. Clones retain the same seal and therefore cannot enter ordinary
/// apply after the transfer. The custody value itself is intentionally
/// non-Clone and exposes only the typed state needed by the root host owner.
pub(crate) struct HostLifecycleCustody {
    plan: LifecyclePlan,
    pre_effect_record: HostLifecycleRecord,
    effect_cursor: usize,
}

impl HostLifecycleCustody {
    pub(crate) fn take(plan: LifecyclePlan) -> Result<Self, LifecycleError> {
        validate_plan(&plan)?;
        plan.authorization_seal.transfer_to_host()?;
        Ok(Self {
            pre_effect_record: HostLifecycleRecord {
                schema_version: "HarnessPluginHostLifecycleRecord-v1",
                plan_id: plan.plan_id.clone(),
                intent: plan.intent,
                before: plan.before.clone(),
                expected_after: plan.expected_after.clone(),
                effects: plan.effects.clone(),
                writes_host_state: plan.writes_host_state,
            },
            plan,
            effect_cursor: 0,
        })
    }

    pub(crate) fn begin_effects(&mut self) -> Result<&LifecyclePlan, LifecycleError> {
        if self.effect_cursor != 0 {
            return Err(LifecycleError::ReplayedPlan);
        }
        self.plan.authorization_seal.consume_transferred()?;
        self.effect_cursor = 1;
        Ok(&self.plan)
    }

    pub(crate) fn plan_id(&self) -> &str {
        &self.plan.plan_id
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

    pub(crate) fn writes_host_state(&self) -> bool {
        self.plan.writes_host_state
    }

    pub(crate) fn finish(
        &mut self,
        recovery_state: Option<&LifecycleState>,
    ) -> Result<(), LifecycleError> {
        if self.effect_cursor != 1 {
            return Err(LifecycleError::InvalidTransition);
        }
        self.plan.authorization_seal.finish_apply(recovery_state)?;
        self.effect_cursor = 2;
        Ok(())
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
                target: Some(authority("b", "1.0.1")),
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
        let mut custody = HostLifecycleCustody::take(plan).unwrap();
        let mut effects = NeverExecute;

        assert_eq!(custody.pre_effect_record().plan_id, custody.plan_id());
        assert_eq!(custody.pre_effect_record().before, before);
        assert_eq!(custody.pre_effect_record().effects, custody.effects());
        assert_eq!(
            apply(&before, &replay, &mut effects),
            Err(LifecycleError::ReplayedPlan)
        );
        assert!(custody.begin_effects().is_ok());
        let expected_after = custody.expected_after().clone();
        assert!(custody.finish(Some(&expected_after)).is_ok());
        assert_eq!(custody.begin_effects(), Err(LifecycleError::ReplayedPlan));
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
