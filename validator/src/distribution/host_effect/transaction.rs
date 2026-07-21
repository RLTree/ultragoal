use super::lifecycle::{
    AcceptedHostScope, HostEffectAcceptanceRequest, HostEffectPreparationRequest,
    SupportedHostLifecycleCoordinator,
};
use super::transaction_failure::{execution_failure, preparation_failure};
use super::transaction_identity::expected_observations;
use super::transaction_observation::{
    HostLifecycleObservationInput, HostLifecycleSurfaceDigests, expected_content, observe,
};
use super::transaction_policy;
use super::transaction_policy::{CurrentDescriptorAdapter, SystemTrustedClock, accepted_lifecycle};
use super::transaction_read_only::observe_read_only;
use super::transaction_recovery::ObservedRecoveryAdapter;
use super::{
    ConfinedHostEffectTarget, DurableHostEffectLedger, FileHostEffectLedger,
    HostEffectCancellation, HostEffectCompletion, NativeRetainedDescriptorProcessBackend,
    SelectedCodexExecutable, SupportedHostEffectExecutor,
};
use crate::distribution::{HostCapabilityDeclaration, JourneyBinding, PackageIdentity};
use crate::plugin_product::lifecycle::{
    HostLifecycleBinding, HostLifecycleCustody, LifecycleIntent, LifecyclePlan,
};
use std::path::Path;

pub(crate) struct HostLifecycleTransactionResult {
    pub(crate) surfaces: HostLifecycleSurfaceDigests,
}

pub(crate) fn execute_host_lifecycle_transaction(
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
    observation: HostLifecycleObservationInput,
) -> Result<HostLifecycleTransactionResult, &'static str> {
    if matches!(
        plan.intent,
        LifecycleIntent::RepeatUse | LifecycleIntent::IdempotentReinstall
    ) {
        let content = expected_content(&observation, target_root)?;
        let expected = expected_observations(&package, &plan, &observation, &content, 0)?;
        let surfaces = observe_read_only(
            &package,
            &plan,
            &command_plan,
            executable,
            target_root,
            observation,
            expected,
        )?;
        return Ok(HostLifecycleTransactionResult { surfaces });
    }
    let content = expected_content(&observation, target_root)?;
    let expected =
        expected_observations(&package, &plan, &observation, &content, command_plan.len())?;
    let scope = AcceptedHostScope::repository(&journey, journey.marketplace().to_owned())
        .map_err(|_| "repository host scope unavailable")?;
    let binding = HostLifecycleBinding::new(
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
    let observation_executable = executable
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
            executable: &executable,
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
            executable,
            target: &mut target_observer,
            clock: &mut clock,
            adapter: &mut adapter,
        })
        .map_err(preparation_failure)?;
    let mut backend = NativeRetainedDescriptorProcessBackend;
    let environment = command_plan
        .commands()
        .first()
        .map(|command| command.environment())
        .ok_or("host lifecycle command plan empty")?;
    if command_plan
        .commands()
        .iter()
        .any(|command| command.environment() != environment)
    {
        return Err("host lifecycle command environments diverged");
    }
    let policy = transaction_policy::isolated_codex_policy(environment)?;
    let cancellation = HostEffectCancellation::default();
    let mut executor =
        SupportedHostEffectExecutor::new(&ledger, target, &mut backend, policy.clone());
    let receipt = executor
        .execute_handoff(handoff, &mut clock, &cancellation)
        .map_err(execution_failure)?;
    let capability = transaction_policy::current_capability()
        .map_err(|_| "host lifecycle observation capability unavailable")?;
    let observation_package = custody.pre_effect_record().package().clone();
    let result = observe(
        &observation_package,
        custody.plan(),
        &observation,
        custody.pre_effect_record().expected_observations(),
        &observation_executable,
        &capability,
        &mut backend,
        &policy,
        &cancellation,
        observation_target.cwd_fd(),
        target_root,
        environment,
        receipt.command_output_sha256().to_vec(),
    )?;
    let ambiguous = result.completed_effects.len() != custody.plan().effects.len();
    let observed = result.observed.clone();
    let completion = if ambiguous {
        HostEffectCompletion::ambiguous(
            custody.completion_binding(),
            observed.clone(),
            result.completed_effects,
            result.observations,
            result.effect_cursor,
        )
    } else {
        HostEffectCompletion::settled(
            custody.completion_binding(),
            observed.clone(),
            result.completed_effects,
            result.observations,
            result.effect_cursor,
        )
    };
    custody
        .settle(completion)
        .map_err(|_| "host lifecycle independent observation settlement failed")?;
    if ambiguous {
        let recovery_authority = custody
            .recovery_token()
            .map_err(|_| "host lifecycle recovery authority unavailable")?;
        if recovery_authority.prior.installed == observed.installed
            && recovery_authority.prior.cache == observed.cache
        {
            let mut observed_recovery = ObservedRecoveryAdapter {
                observed: observed.clone(),
            };
            crate::plugin_product::lifecycle::execution::recover(
                &observed,
                &recovery_authority,
                &mut observed_recovery,
            )
            .map_err(|_| "host lifecycle recovery authorization failed")?;
        }
    }
    Ok(HostLifecycleTransactionResult {
        surfaces: result.surfaces,
    })
}
