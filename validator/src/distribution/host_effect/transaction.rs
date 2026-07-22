use super::transaction_failure::{execution_failure, preparation_failure};
use super::transaction_identity::expected_observations;
use super::transaction_observation::{
    HostLifecycleObservationInput, HostLifecycleSurfaceDigests, expected_content, observe,
};
use super::transaction_policy;
use super::transaction_preparation::prepare_host_effect_transaction;
use super::transaction_read_only::observe_read_only;
use super::transaction_recovery::ObservedRecoveryAdapter;
use super::{
    HostEffectCancellation, HostEffectCompletion, NativeRetainedDescriptorProcessBackend,
    SelectedCodexExecutable, SupportedHostEffectExecutor,
};
use crate::distribution::{HostCapabilityDeclaration, JourneyBinding, PackageIdentity};
use crate::plugin_product::lifecycle::{LifecycleIntent, LifecyclePlan};
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
        let expected = match (|| {
            let content = expected_content(&observation, target_root)?;
            expected_observations(&package, &plan, &observation, &content, 0)
        })() {
            Ok(expected) => expected,
            Err(error) => return abort_without_effect(executable, error),
        };
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
    let mut prepared = prepare_host_effect_transaction(
        plan,
        package,
        command_plan,
        journey,
        host,
        ledger_root,
        ledger_id,
        issuer_id,
        executable,
        target_root,
        &observation,
    )?;
    let mut backend = NativeRetainedDescriptorProcessBackend;
    let cancellation = HostEffectCancellation::default();
    let mut executor = SupportedHostEffectExecutor::new(
        &prepared.ledger,
        prepared.target,
        &mut backend,
        prepared.policy.clone(),
    );
    let (handoff, receipt) = executor
        .execute_handoff(prepared.handoff, &mut prepared.clock, &cancellation)
        .into_parts();
    let receipt = match receipt {
        Ok(receipt) => receipt,
        Err(error) => {
            // Recovery remains unresolved: dropping this opaque owner is
            // intentionally non-destructive, so the staged executable stays
            // preserved for the durable recovery path rather than being
            // cleaned up on an ambiguous execution result.
            drop(handoff);
            return Err(execution_failure(error));
        }
    };
    let capability = transaction_policy::current_capability()
        .map_err(|_| "host lifecycle observation capability unavailable")?;
    let observation_package = prepared.custody.pre_effect_record().package().clone();
    let result = observe(
        &observation_package,
        prepared.custody.plan(),
        &observation,
        prepared.custody.pre_effect_record().expected_observations(),
        &prepared.observation_executable,
        &capability,
        &mut backend,
        &prepared.policy,
        &cancellation,
        prepared.observation_target.cwd_fd(),
        target_root,
        &prepared.environment,
        receipt.command_output_sha256().to_vec(),
    )?;
    // On Darwin the observation duplicate shares the staged private object.
    // It must be released before the settled handoff can prove sole ownership
    // and perform explicit finalization.
    drop(prepared.observation_executable);
    let ambiguous = result.completed_effects.len() != prepared.custody.plan().effects.len();
    let observed = result.observed.clone();
    let completion = if ambiguous {
        HostEffectCompletion::ambiguous(
            prepared.custody.completion_binding(),
            observed.clone(),
            result.completed_effects,
            result.observations,
            result.effect_cursor,
        )
    } else {
        HostEffectCompletion::settled(
            prepared.custody.completion_binding(),
            observed.clone(),
            result.completed_effects,
            result.observations,
            result.effect_cursor,
        )
    };
    prepared
        .custody
        .settle(completion)
        .map_err(|_| "host lifecycle independent observation settlement failed")?;
    if ambiguous {
        let mut observed_recovery = ObservedRecoveryAdapter {
            observed: observed.clone(),
        };
        prepared
            .custody
            .recover(&observed, &mut observed_recovery)
            .map_err(|_| "host lifecycle recovery authorization failed")?;
    }
    let finalization = prepared
        .custody
        .finalization_token()
        .map_err(|_| "host lifecycle terminal finalization unavailable")?;
    handoff
        .finalize(finalization)
        .map_err(preparation_failure)?;
    Ok(HostLifecycleTransactionResult {
        surfaces: result.surfaces,
    })
}

fn abort_without_effect<T>(
    executable: SelectedCodexExecutable,
    error: &'static str,
) -> Result<T, &'static str> {
    executable
        .finalize()
        .map_err(|_| "host executable pre-effect finalization failed")?;
    Err(error)
}
