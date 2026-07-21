use super::transaction_observation::{
    HostLifecycleObservationInput, HostLifecycleSurfaceDigests, observe,
};
use super::transaction_observation_transition::validate_read_only_transition;
use super::transaction_policy;
use super::{HostEffectCancellation, NativeRetainedDescriptorProcessBackend, PinnedHostExecutable};
use crate::distribution::PackageIdentity;
use crate::plugin_product::lifecycle::{HostLifecycleExpectedObservations, LifecyclePlan};
use std::os::fd::AsRawFd;
use std::path::Path;

pub(super) fn observe_read_only(
    package: &PackageIdentity,
    plan: &LifecyclePlan,
    command_plan: &super::HostCommandPlan,
    executable: PinnedHostExecutable,
    target_root: &Path,
    observation: HostLifecycleObservationInput,
    expected: HostLifecycleExpectedObservations,
) -> Result<HostLifecycleSurfaceDigests, &'static str> {
    let root = std::fs::File::open(target_root).map_err(|_| "host observation root unavailable")?;
    let environment = command_plan
        .commands()
        .first()
        .map(|command| command.environment())
        .ok_or("host lifecycle command plan empty")?;
    let policy = transaction_policy::isolated_codex_policy(environment)?;
    let mut backend = NativeRetainedDescriptorProcessBackend;
    let cancellation = HostEffectCancellation::default();
    let capability = transaction_policy::current_capability()
        .map_err(|_| "host lifecycle observation capability unavailable")?;
    let result = observe(
        package,
        plan,
        &observation,
        &expected,
        &executable,
        &capability,
        &mut backend,
        &policy,
        &cancellation,
        root.as_raw_fd(),
        target_root,
        environment,
        Vec::new(),
    )?;
    if result.completed_effects.len() != plan.effects.len() {
        return Err("read-only lifecycle observations incomplete");
    }
    validate_read_only_transition(plan, &result)?;
    Ok(result.surfaces)
}
