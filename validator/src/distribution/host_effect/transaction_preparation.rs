use super::lifecycle::{
    AcceptedHostScope, HostEffectAcceptanceRequest, HostEffectPreparationRequest,
    SupportedHostLifecycleCoordinator,
};
use super::transaction_failure::preparation_failure;
use super::transaction_identity::expected_observations;
use super::transaction_observation::{HostLifecycleObservationInput, expected_content};
use super::transaction_policy::{
    self, CurrentDescriptorAdapter, SystemTrustedClock, accepted_lifecycle,
};
use super::{
    ConfinedHostEffectTarget, DurableHostEffectLedger, FileHostEffectLedger,
    HostEffectExecutionPolicy, SelectedCodexExecutable,
};
use crate::distribution::{HostCapabilityDeclaration, JourneyBinding, PackageIdentity};
use crate::plugin_product::lifecycle::{HostLifecycleCustody, LifecycleIntent, LifecyclePlan};
use std::path::Path;

pub(super) struct PreparedHostEffectTransaction {
    pub(super) custody: HostLifecycleCustody,
    pub(super) ledger: FileHostEffectLedger,
    pub(super) target: ConfinedHostEffectTarget,
    pub(super) observation_target: ConfinedHostEffectTarget,
    pub(super) handoff: super::lifecycle::DescriptorExecutionHandoff,
    pub(super) policy: HostEffectExecutionPolicy,
    pub(super) environment: Vec<(String, String)>,
    pub(super) clock: SystemTrustedClock,
}

pub(super) fn prepare_host_effect_transaction(
    plan: LifecyclePlan,
    package: PackageIdentity,
    command_plan: super::HostCommandPlan,
    journey: JourneyBinding,
    host: HostCapabilityDeclaration,
    ledger_root: &Path,
    ledger_id: String,
    issuer_id: String,
    executable: SelectedCodexExecutable,
    target_root: &Path,
    observation: &HostLifecycleObservationInput,
) -> Result<PreparedHostEffectTransaction, &'static str> {
    let mut selected = Some(executable);
    let result = (|| {
        let content = expected_content(observation, target_root)?;
        let command_count = if matches!(
            plan.intent,
            LifecycleIntent::RepeatUse | LifecycleIntent::IdempotentReinstall
        ) {
            0
        } else {
            command_plan.len()
        };
        let expected =
            expected_observations(&package, &plan, observation, &content, command_count)?;
        let scope = AcceptedHostScope::repository(&journey, journey.marketplace().to_owned())
            .map_err(|_| "repository host scope unavailable")?;
        scope
            .validate_plan(
                &package,
                transaction_policy::accepted_operation(plan.intent),
                &command_plan,
            )
            .map_err(|_| "host command plan admission failed")?;
        let binding = crate::plugin_product::lifecycle::HostLifecycleBinding::new(
            package.clone(),
            command_plan,
            scope.binding_sha256().map_err(|_| "scope binding failed")?,
            host.capability_sha256().to_owned(),
            expected,
        )
        .map_err(|_| "host lifecycle binding failed")?;
        let mut custody = HostLifecycleCustody::take(plan, binding)
            .map_err(|_| "host lifecycle custody admission failed")?;
        let ledger = FileHostEffectLedger::create(ledger_root, ledger_id.clone())
            .map_err(|_| "host lifecycle ledger creation failed")?;
        let coordinator = SupportedHostLifecycleCoordinator::bind(issuer_id, ledger_id, &ledger)
            .map_err(|_| "host lifecycle coordinator binding failed")?;
        let (target, expected_target) = ConfinedHostEffectTarget::bind(
            target_root,
            scope.clone(),
            custody.expected_after().generation,
        )
        .map_err(|_| "host target binding failed")?;
        let observation_target = target.clone();
        let lifecycle = accepted_lifecycle(&custody, &package)?;
        let projection = custody.command_plan_projection();
        let environment = projection.environment().to_vec();
        let policy = transaction_policy::isolated_codex_policy(&environment)?;
        let expected_head = ledger
            .head()
            .map_err(|_| "host lifecycle ledger head unavailable")?;
        let accepted = coordinator
            .accept(HostEffectAcceptanceRequest {
                package,
                journey,
                host,
                lifecycle,
                scope,
                plan: projection,
                executable: selected
                    .as_ref()
                    .ok_or("host executable selection unavailable")?,
                expected_target,
                expected_head,
                lifecycle_record: custody.pre_effect_record(),
            })
            .map_err(|_| "host lifecycle identity admission failed")?;
        let mut target_observer = target.observer();
        let mut adapter = CurrentDescriptorAdapter;
        let mut clock = SystemTrustedClock::default();
        let handoff = coordinator
            .prepare_current(HostEffectPreparationRequest {
                accepted: &accepted,
                custody: &mut custody,
                executable: selected
                    .take()
                    .ok_or("host executable selection unavailable")?,
                target: &mut target_observer,
                clock: &mut clock,
                adapter: &mut adapter,
            })
            .map_err(preparation_failure)?;
        Ok(PreparedHostEffectTransaction {
            custody,
            ledger,
            target,
            observation_target,
            handoff,
            policy,
            environment,
            clock,
        })
    })();
    match result {
        Ok(prepared) => Ok(prepared),
        Err(error) => {
            if let Some(selected) = selected {
                return Err(preserve_pre_effect_refusal(error, || selected.finalize()));
            }
            Err(error)
        }
    }
}

fn preserve_pre_effect_refusal<E>(
    error: &'static str,
    cleanup: impl FnOnce() -> Result<(), E>,
) -> &'static str {
    let _ = cleanup();
    error
}

#[cfg(test)]
mod tests {
    #[test]
    fn cleanup_failure_does_not_replace_pre_effect_refusal() {
        assert_eq!(
            super::preserve_pre_effect_refusal("target refusal", || Err(())),
            "target refusal"
        );
    }
}
