use super::transaction_carrier::{HostLifecycleRecoveryCause, HostLifecycleTransactionOutcome};
use super::transaction_failure::preparation_failure;
use super::transaction_observation::{HostLifecycleObservationInput, observe};
use super::transaction_observation_transition::validate_read_only_transition;
use super::transaction_policy;
use super::transaction_preparation::{
    PreparedHostEffectTransaction, prepare_host_effect_transaction,
};
use super::transaction_recovery_carrier::recovery_required;
use super::{
    HostEffectCancellation, HostEffectCompletion, NativeRetainedDescriptorProcessBackend,
    SelectedCodexExecutable, SupportedHostEffectExecutor,
};
use crate::distribution::{HostCapabilityDeclaration, JourneyBinding, PackageIdentity};
use crate::plugin_product::lifecycle::LifecyclePlan;
use std::path::Path;

pub(super) fn execute_read_only_transaction(
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
    let PreparedHostEffectTransaction {
        custody,
        ledger,
        target,
        observation_target,
        mut handoff,
        policy,
        environment,
        mut clock,
    } = prepare_host_effect_transaction(
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
    let mut custody = custody;
    let mut backend = NativeRetainedDescriptorProcessBackend;
    let cancellation = HostEffectCancellation::default();
    let capability = match transaction_policy::current_capability() {
        Ok(capability) => capability,
        Err(_) => {
            return Ok(recovery_required(
                handoff,
                custody,
                ledger,
                observation_target,
                observation,
                target_root,
                policy,
                environment,
                None,
                Vec::new(),
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
            Vec::new(),
        )
    }) {
        Ok(Ok(result)) => result,
        Ok(Err(error)) => {
            return Ok(recovery_required(
                handoff,
                custody,
                ledger,
                observation_target,
                observation,
                target_root,
                policy,
                environment,
                None,
                Vec::new(),
                HostLifecycleRecoveryCause::Observation(error),
            ));
        }
        Err(_) => {
            return Ok(recovery_required(
                handoff,
                custody,
                ledger,
                observation_target,
                observation,
                target_root,
                policy,
                environment,
                None,
                Vec::new(),
                HostLifecycleRecoveryCause::Observation(
                    "host lifecycle observation executable revalidation failed",
                ),
            ));
        }
    };
    if result.completed_effects.len() != custody.plan().effects.len() {
        return Ok(recovery_required(
            handoff,
            custody,
            ledger,
            observation_target,
            observation,
            target_root,
            policy,
            environment,
            None,
            Vec::new(),
            HostLifecycleRecoveryCause::Observation("read-only lifecycle observations incomplete"),
        ));
    }
    if let Err(error) = validate_read_only_transition(custody.plan(), &result) {
        return Ok(recovery_required(
            handoff,
            custody,
            ledger,
            observation_target,
            observation,
            target_root,
            policy,
            environment,
            None,
            Vec::new(),
            HostLifecycleRecoveryCause::Observation(error),
        ));
    }
    let executor = SupportedHostEffectExecutor::new(&ledger, target, &mut backend, policy.clone());
    if handoff
        .with_retained_lifecycle(|capability, effect, lease, _| {
            executor.settle_observation(capability, effect, lease, &mut clock)
        })
        .is_err()
    {
        return Ok(recovery_required(
            handoff,
            custody,
            ledger,
            observation_target,
            observation,
            target_root,
            policy,
            environment,
            None,
            Vec::new(),
            HostLifecycleRecoveryCause::Settlement(
                "host lifecycle durable observation settlement failed",
            ),
        ));
    }
    drop(executor);
    let observed = result.observed.clone();
    let completion = HostEffectCompletion::settled(
        custody.completion_binding(),
        observed,
        result.completed_effects,
        result.observations,
        result.effect_cursor,
    );
    if custody.settle(completion).is_err() {
        return Ok(recovery_required(
            handoff,
            custody,
            ledger,
            observation_target,
            observation,
            target_root,
            policy,
            environment,
            None,
            Vec::new(),
            HostLifecycleRecoveryCause::Settlement(
                "host lifecycle custody observation settlement failed",
            ),
        ));
    }
    let finalization = custody
        .finalization_token()
        .map_err(|_| "host lifecycle terminal finalization unavailable")?;
    if let Err(error) = handoff.finalize(finalization) {
        return Ok(HostLifecycleTransactionOutcome::FinalizedFailure(
            preparation_failure(error),
        ));
    }
    Ok(HostLifecycleTransactionOutcome::Completed(
        super::transaction::HostLifecycleTransactionResult {
            surfaces: result.surfaces,
        },
    ))
}
