use super::transaction_carrier::{HostLifecycleRecoveryCause, HostLifecycleTransactionOutcome};
use super::transaction_failure::execution_failure;
use super::transaction_observation::{HostLifecycleObservationInput, observe};
use super::transaction_policy;
use super::transaction_preparation::{
    PreparedHostEffectTransaction, prepare_host_effect_transaction,
};
use super::transaction_recovery::ObservedRecoveryAdapter;
use super::transaction_recovery_carrier::recovery_required;
use super::{
    HostEffectCancellation, HostEffectCompletion, NativeRetainedDescriptorProcessBackend,
    SelectedCodexExecutable, SupportedHostEffectExecutor,
};
use crate::distribution::{HostCapabilityDeclaration, JourneyBinding, PackageIdentity};
use crate::plugin_product::lifecycle::LifecyclePlan;
use std::path::Path;

pub(super) fn execute_effectful_transaction(
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
) -> Result<HostLifecycleTransactionOutcome, &'static str> {
    let prepared = prepare_host_effect_transaction(
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
    let PreparedHostEffectTransaction {
        custody,
        ledger,
        target,
        observation_target,
        handoff: initial_handoff,
        policy,
        environment,
        mut clock,
    } = prepared;
    let mut custody = custody;
    let mut backend = NativeRetainedDescriptorProcessBackend;
    let cancellation = HostEffectCancellation::default();
    let mut executor =
        SupportedHostEffectExecutor::new(&ledger, target, &mut backend, policy.clone());
    let (handoff, receipt) = executor
        .execute_handoff(initial_handoff, &mut clock, &cancellation)
        .into_parts();
    drop(executor);
    let receipt = match receipt {
        Ok(receipt) => receipt,
        Err(error) => {
            let recovery = error.recovery().cloned();
            return Ok(recovery_required(
                handoff,
                custody,
                ledger,
                observation_target,
                observation.clone(),
                target_root,
                policy,
                environment,
                recovery,
                Vec::new(),
                HostLifecycleRecoveryCause::Execution(execution_failure(error)),
            ));
        }
    };
    let receipt_recovery = Some(receipt.recovery_handoff());
    let capability = match transaction_policy::current_capability() {
        Ok(capability) => capability,
        Err(_) => {
            return Ok(recovery_required(
                handoff,
                custody,
                ledger,
                observation_target,
                observation.clone(),
                target_root,
                policy,
                environment,
                receipt_recovery.clone(),
                receipt.command_output_sha256().to_vec(),
                HostLifecycleRecoveryCause::Observation(
                    "host lifecycle observation capability unavailable",
                ),
            ));
        }
    };
    let observation_package = custody.pre_effect_record().package().clone();
    let result = match handoff.with_revalidated_observation(|observation_executable| {
        observe(
            &observation_package,
            custody.plan(),
            &observation,
            custody.pre_effect_record().expected_observations(),
            observation_executable,
            &capability,
            &mut backend,
            &policy,
            &cancellation,
            observation_target.cwd_fd(),
            target_root,
            &environment,
            receipt.command_output_sha256().to_vec(),
        )
    }) {
        Ok(Ok(result)) => result,
        Ok(Err(error)) => {
            return Ok(recovery_required(
                handoff,
                custody,
                ledger,
                observation_target,
                observation.clone(),
                target_root,
                policy,
                environment,
                receipt_recovery.clone(),
                receipt.command_output_sha256().to_vec(),
                HostLifecycleRecoveryCause::Observation(error),
            ));
        }
        Err(_) => {
            return Ok(recovery_required(
                handoff,
                custody,
                ledger,
                observation_target,
                observation.clone(),
                target_root,
                policy,
                environment,
                receipt_recovery.clone(),
                receipt.command_output_sha256().to_vec(),
                HostLifecycleRecoveryCause::Observation(
                    "host lifecycle observation executable revalidation failed",
                ),
            ));
        }
    };
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
    if custody.settle(completion).is_err() {
        return Ok(recovery_required(
            handoff,
            custody,
            ledger,
            observation_target,
            observation.clone(),
            target_root,
            policy,
            environment,
            receipt_recovery.clone(),
            receipt.command_output_sha256().to_vec(),
            HostLifecycleRecoveryCause::Settlement(
                "host lifecycle independent observation settlement failed",
            ),
        ));
    }
    if ambiguous {
        let mut observed_recovery = ObservedRecoveryAdapter {
            observed: observed.clone(),
        };
        if custody.recover(&observed, &mut observed_recovery).is_err() {
            return Ok(recovery_required(
                handoff,
                custody,
                ledger,
                observation_target,
                observation.clone(),
                target_root,
                policy,
                environment,
                receipt_recovery.clone(),
                receipt.command_output_sha256().to_vec(),
                HostLifecycleRecoveryCause::Settlement(
                    "host lifecycle recovery authorization failed",
                ),
            ));
        }
    }
    let finalization = match custody.finalization_token() {
        Ok(finalization) => finalization,
        Err(_) => {
            return Ok(recovery_required(
                handoff,
                custody,
                ledger,
                observation_target,
                observation.clone(),
                target_root,
                policy,
                environment,
                receipt_recovery,
                receipt.command_output_sha256().to_vec(),
                HostLifecycleRecoveryCause::Finalization(
                    "host lifecycle terminal finalization unavailable",
                ),
            ));
        }
    };
    if let Err(error) = handoff.finalize(finalization) {
        return Ok(HostLifecycleTransactionOutcome::FinalizedFailure(
            super::transaction_failure::preparation_failure(error),
        ));
    }
    Ok(HostLifecycleTransactionOutcome::Completed(
        super::transaction::HostLifecycleTransactionResult {
            surfaces: result.surfaces,
        },
    ))
}
