use super::PinnedHostExecutable;
use super::transaction_identity::{identity, normalized_json, semantic_digest};
use super::transaction_observation_command::{observation_command, run_json, validate_input};
use super::transaction_observation_transition::{
    SurfacePresence, derive_effect_prefix, expected_presence,
};
use super::transaction_tree::{observe_file, observe_tree};
use crate::distribution::PackageIdentity;
use crate::distribution::host_effect::executor::{
    HostEffectCancellation, HostEffectExecutionPolicy, NativeRetainedDescriptorProcessBackend,
};
use crate::distribution::host_effect::lifecycle::DescriptorExecutionCapability;
use crate::plugin_product::lifecycle::{
    HostCommandObservation, HostLifecycleExpectedObservations, HostLifecycleObservedBundle,
};
use crate::plugin_product::lifecycle::{LifecycleEffect, LifecyclePlan, LifecycleState};
use std::os::fd::RawFd;
use std::path::{Path, PathBuf};

pub(crate) struct HostLifecycleObservationInput {
    pub(crate) installed_path: PathBuf,
    pub(crate) cache_path: PathBuf,
    pub(crate) runtime_path: PathBuf,
    pub(crate) marketplace: String,
    pub(crate) plugin: String,
}

pub(crate) struct HostLifecycleSurfaceDigests {
    pub(crate) installed: String,
    pub(crate) cache: String,
    pub(crate) registry: String,
    pub(crate) discovery: String,
    pub(crate) runtime: String,
}

pub(crate) struct HostLifecycleObservationResult {
    pub(crate) observations: HostLifecycleObservedBundle,
    pub(crate) completed_effects: Vec<LifecycleEffect>,
    pub(crate) observed: LifecycleState,
    pub(crate) effect_cursor: usize,
    pub(crate) surfaces: HostLifecycleSurfaceDigests,
}

pub(crate) fn observe(
    package: &PackageIdentity,
    plan: &LifecyclePlan,
    input: &HostLifecycleObservationInput,
    expected: &HostLifecycleExpectedObservations,
    executable: &PinnedHostExecutable,
    capability: &DescriptorExecutionCapability,
    backend: &mut NativeRetainedDescriptorProcessBackend,
    policy: &HostEffectExecutionPolicy,
    cancellation: &HostEffectCancellation,
    cwd: RawFd,
    target_root: &Path,
    environment: &[(String, String)],
    command_digests: Vec<String>,
) -> Result<HostLifecycleObservationResult, &'static str> {
    validate_input(input, target_root, cwd)?;
    let marketplace_json = run_json(
        backend,
        capability,
        executable,
        policy,
        cancellation,
        cwd,
        observation_command(&["plugin", "marketplace", "list", "--json"], environment),
    )?;
    let plugin_json = run_json(
        backend,
        capability,
        executable,
        policy,
        cancellation,
        cwd,
        observation_command(&["plugin", "list", "--json"], environment),
    )?;
    let installed = observe_tree(&input.installed_path)?;
    let cache = observe_tree(&input.cache_path)?;
    let runtime = observe_file(&input.runtime_path)?;
    let marketplace_semantic = normalized_json(&marketplace_json, &input.marketplace);
    let plugin_semantic = normalized_json(&plugin_json, &input.plugin);
    let installed_identity = identity(
        "installed",
        package,
        plan,
        input,
        plan.expected_after.installed.as_ref(),
    );
    let cache_identity = identity(
        "cache",
        package,
        plan,
        input,
        plan.expected_after.cache.as_ref(),
    );
    let registry_identity = identity(
        "registry",
        package,
        plan,
        input,
        plan.expected_after.installed.as_ref(),
    );
    let discovery_identity = identity(
        "discovery",
        package,
        plan,
        input,
        plan.expected_after.installed.as_ref(),
    );
    let runtime_identity = identity(
        "runtime",
        package,
        plan,
        input,
        plan.expected_after.installed.as_ref(),
    );
    let expected_identity = [
        &expected.installed_sha256,
        &expected.cache_sha256,
        &expected.registry_sha256,
        &expected.discovery_sha256,
        &expected.runtime_sha256,
    ];
    let observed_identity = [
        &installed_identity,
        &cache_identity,
        &registry_identity,
        &discovery_identity,
        &runtime_identity,
    ];
    if SurfacePresence::from_path(installed.present)
        != expected_presence(plan.expected_after.installed.as_ref())
        || SurfacePresence::from_path(cache.present)
            != expected_presence(plan.expected_after.cache.as_ref())
        || SurfacePresence::from_json(marketplace_semantic.as_ref())
            != expected_presence(plan.expected_after.installed.as_ref())
        || SurfacePresence::from_json(plugin_semantic.as_ref())
            != expected_presence(plan.expected_after.installed.as_ref())
        || SurfacePresence::from_path(runtime.present)
            != expected_presence(plan.expected_after.installed.as_ref())
    {
        return Err("host surface presence does not match candidate authority");
    }
    if expected_identity != observed_identity {
        return Err("host observations do not match the candidate identity");
    }
    let commands = command_digests
        .into_iter()
        .enumerate()
        .map(|(index, digest)| HostCommandObservation::new(index, digest, 0, 1))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "host command observation binding failed")?;
    let observations = HostLifecycleObservedBundle::from_parts(
        installed_identity.clone(),
        cache_identity.clone(),
        registry_identity.clone(),
        discovery_identity.clone(),
        runtime_identity.clone(),
        commands,
    )
    .map_err(|_| "host observation bundle invalid")?;
    let completed_effects = derive_effect_prefix(
        plan,
        SurfacePresence::from_path(installed.present),
        SurfacePresence::from_path(cache.present),
        SurfacePresence::from_path(runtime.present),
        SurfacePresence::from_json(marketplace_semantic.as_ref()),
        SurfacePresence::from_json(plugin_semantic.as_ref()),
    )?;
    let observed = if completed_effects.len() == plan.effects.len() {
        plan.expected_after.clone()
    } else {
        crate::plugin_product::lifecycle::recovery_state_after_completed_prefix(
            plan,
            &completed_effects,
        )
        .map_err(|_| "host recovery prefix unavailable")?
    };
    Ok(HostLifecycleObservationResult {
        observations,
        completed_effects: completed_effects.clone(),
        observed,
        effect_cursor: completed_effects.len(),
        surfaces: HostLifecycleSurfaceDigests {
            installed: installed.digest,
            cache: cache.digest,
            registry: semantic_digest(&marketplace_semantic),
            discovery: semantic_digest(&plugin_semantic),
            runtime: runtime.digest,
        },
    })
}
