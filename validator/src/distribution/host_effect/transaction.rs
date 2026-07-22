use super::transaction_failure::{execution_failure, preparation_failure};
use super::transaction_identity::expected_observations;
use super::transaction_observation::{
    HostLifecycleObservationInput, HostLifecycleSurfaceDigests, expected_content, observe,
};
use super::transaction_policy;
use super::transaction_preparation::prepare_host_effect_transaction;
use super::transaction_read_only::observe_read_only;
use super::transaction_recovery::ObservedRecoveryAdapter;
#[cfg(not(test))]
use super::transaction_recovery::finalize_recorded_recovery;
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
            #[cfg(not(test))]
            {
                let recovery = error
                    .recovery()
                    .filter(|recovery| recovery.terminal_state().is_some())
                    .cloned();
                if let Some(recovery) = recovery {
                    return finalize_recorded_recovery(
                        handoff,
                        &mut prepared.custody,
                        &recovery,
                        execution_failure(error),
                    );
                }
                handoff.retain_for_recovery();
                return Err(execution_failure(error));
            }
            #[cfg(test)]
            {
                let _ = handoff;
                return Err(execution_failure(error));
            }
        }
    };
    #[cfg(not(test))]
    let receipt_recovery = receipt.recovery_handoff();
    let capability = match transaction_policy::current_capability() {
        Ok(capability) => capability,
        Err(_) => {
            #[cfg(not(test))]
            {
                return finalize_recorded_recovery(
                    handoff,
                    &mut prepared.custody,
                    &receipt_recovery,
                    "host lifecycle observation capability unavailable",
                );
            }
            #[cfg(test)]
            return Err("host lifecycle observation capability unavailable");
        }
    };
    let observation_package = prepared.custody.pre_effect_record().package().clone();
    let result = match observe(
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
    ) {
        Ok(result) => result,
        Err(error) => {
            #[cfg(not(test))]
            {
                return finalize_recorded_recovery(
                    handoff,
                    &mut prepared.custody,
                    &receipt_recovery,
                    error,
                );
            }
            #[cfg(test)]
            return Err(error);
        }
    };
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
    if prepared.custody.settle(completion).is_err() {
        #[cfg(not(test))]
        {
            return finalize_recorded_recovery(
                handoff,
                &mut prepared.custody,
                &receipt_recovery,
                "host lifecycle independent observation settlement failed",
            );
        }
        #[cfg(test)]
        return Err("host lifecycle independent observation settlement failed");
    }
    if ambiguous {
        let mut observed_recovery = ObservedRecoveryAdapter {
            observed: observed.clone(),
        };
        if prepared
            .custody
            .recover(&observed, &mut observed_recovery)
            .is_err()
        {
            #[cfg(not(test))]
            {
                return finalize_recorded_recovery(
                    handoff,
                    &mut prepared.custody,
                    &receipt_recovery,
                    "host lifecycle recovery authorization failed",
                );
            }
            #[cfg(test)]
            return Err("host lifecycle recovery authorization failed");
        }
    }
    let finalization = match prepared.custody.finalization_token() {
        Ok(finalization) => finalization,
        Err(_) => {
            #[cfg(not(test))]
            {
                return finalize_recorded_recovery(
                    handoff,
                    &mut prepared.custody,
                    &receipt_recovery,
                    "host lifecycle terminal finalization unavailable",
                );
            }
            #[cfg(test)]
            return Err("host lifecycle terminal finalization unavailable");
        }
    };
    if let Err(error) = handoff.finalize(finalization) {
        return Err(preparation_failure(error));
    }
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
