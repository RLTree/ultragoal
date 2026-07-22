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
use crate::plugin_product::lifecycle::{HostLifecycleCustody, LifecyclePlan};
use std::path::Path;

pub(super) struct PreparedHostEffectTransaction {
    pub(super) custody: HostLifecycleCustody,
    pub(super) ledger: FileHostEffectLedger,
    pub(super) target: ConfinedHostEffectTarget,
    pub(super) observation_target: ConfinedHostEffectTarget,
    pub(super) observation_executable: SelectedCodexExecutable,
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
        let expected =
            expected_observations(&package, &plan, observation, &content, command_plan.len())?;
        let scope = AcceptedHostScope::repository(&journey, journey.marketplace().to_owned())
            .map_err(|_| "repository host scope unavailable")?;
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
        let observation_executable = selected
            .as_ref()
            .ok_or("host executable selection unavailable")?
            .duplicate()
            .map_err(|_| "host executable duplicate failed")?;
        let (target, expected_target) = ConfinedHostEffectTarget::bind(
            target_root,
            scope.clone(),
            custody.expected_after().generation,
        )
        .map_err(|_| "host target binding failed")?;
        let observation_target = target.clone();
        let lifecycle = accepted_lifecycle(&custody, &package)?;
        let command_plan = custody
            .candidate_plan()
            .map_err(|_| "host command plan unavailable")?;
        let environment = command_plan
            .commands()
            .first()
            .map(|command| command.environment().to_vec())
            .ok_or("host lifecycle command plan empty")?;
        if command_plan
            .commands()
            .iter()
            .any(|command| command.environment() != environment.as_slice())
        {
            return Err("host lifecycle command environments diverged");
        }
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
                plan: &command_plan,
                executable: selected
                    .as_ref()
                    .ok_or("host executable selection unavailable")?,
                expected_target,
                expected_head,
                #[cfg(not(test))]
                lifecycle_record: custody.pre_effect_record(),
                #[cfg(test)]
                lifecycle_record: Some(custody.pre_effect_record()),
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
            observation_executable,
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
                selected
                    .finalize()
                    .map_err(|_| "host executable pre-effect finalization failed")?;
            }
            Err(error)
        }
    }
}
