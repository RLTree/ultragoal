use super::SelectedCodexExecutable;
use super::transaction_carrier::{HostLifecycleTransactionOutcome, resolve_host_lifecycle_outcome};
use super::transaction_effectful::execute_effectful_transaction;
use super::transaction_identity::expected_observations;
use super::transaction_observation::{
    HostLifecycleObservationInput, HostLifecycleSurfaceDigests, expected_content,
};
use super::transaction_read_only::observe_read_only;
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
    let outcome = if matches!(
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
        HostLifecycleTransactionOutcome::Completed(HostLifecycleTransactionResult { surfaces })
    } else {
        execute_effectful_transaction(
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
            observation,
        )?
    };
    resolve_host_lifecycle_outcome(outcome)
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
