use super::SelectedCodexExecutable;
use super::transaction_carrier::resolve_host_lifecycle_outcome;
use super::transaction_effectful::execute_effectful_transaction;
use super::transaction_observation::{HostLifecycleObservationInput, HostLifecycleSurfaceDigests};
use super::transaction_read_only::execute_read_only_transaction;
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
        execute_read_only_transaction(
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
