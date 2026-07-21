use super::SelectedCodexExecutable;
use super::transaction_identity::absent_digest;
use super::transaction_observation_command::{
    observation_command, run_json, validate_input, validate_paths,
};
use super::transaction_observation_identity::{
    HostSurfaceLocations, observe_marketplace, observe_plugin,
};
use super::transaction_observation_transition::{SurfacePresence, derive_effect_prefix};
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
    pub(crate) marketplace_source_path: PathBuf,
    pub(crate) marketplace_source_root: PathBuf,
    pub(crate) marketplace: String,
    pub(crate) plugin: String,
}

pub(crate) struct HostLifecycleExpectedContent {
    pub(crate) installed: String,
    pub(crate) cache: String,
    pub(crate) runtime: String,
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
    _package: &PackageIdentity,
    plan: &LifecyclePlan,
    input: &HostLifecycleObservationInput,
    expected: &HostLifecycleExpectedObservations,
    executable: &SelectedCodexExecutable,
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
    let required = plan.expected_after.installed.is_some();
    let marketplace_identity = observe_marketplace(&marketplace_json, input, required)?;
    let (plugin_identity, locations) = observe_plugin(
        &plugin_json,
        input,
        plan.expected_after.installed.as_ref(),
        target_root,
    )?;
    validate_locations(input, target_root, &locations)?;
    let installed = observe_tree(&locations.installed)?;
    let cache = observe_tree(&locations.cache)?;
    let runtime = observe_file(&locations.runtime)?;
    let observed_installed = if installed.present {
        installed.digest.clone()
    } else {
        absent_digest()
    };
    let observed_cache = if cache.present {
        cache.digest.clone()
    } else {
        absent_digest()
    };
    let observed_runtime = if runtime.present {
        runtime.digest.clone()
    } else {
        absent_digest()
    };
    let observed_registry = marketplace_identity.clone().unwrap_or_else(absent_digest);
    let observed_discovery = plugin_identity.clone().unwrap_or_else(absent_digest);
    if [
        &observed_installed,
        &observed_cache,
        &observed_registry,
        &observed_discovery,
        &observed_runtime,
    ] != [
        &expected.installed_sha256,
        &expected.cache_sha256,
        &expected.registry_sha256,
        &expected.discovery_sha256,
        &expected.runtime_sha256,
    ] {
        return Err("independent host content does not match candidate authority");
    }
    let commands = command_digests
        .into_iter()
        .enumerate()
        .map(|(index, digest)| HostCommandObservation::new(index, digest, 0, 1))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "host command observation binding failed")?;
    let observations = HostLifecycleObservedBundle::from_parts(
        observed_installed.clone(),
        observed_cache.clone(),
        observed_registry.clone(),
        observed_discovery.clone(),
        observed_runtime.clone(),
        commands,
    )
    .map_err(|_| "host observation bundle invalid")?;
    let completed_effects = derive_effect_prefix(
        plan,
        SurfacePresence::from_path(installed.present),
        SurfacePresence::from_path(cache.present),
        SurfacePresence::from_path(runtime.present),
        SurfacePresence::from_json(marketplace_identity.as_ref()),
        SurfacePresence::from_json(plugin_identity.as_ref()),
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
            installed: observed_installed,
            cache: observed_cache,
            registry: observed_registry,
            discovery: observed_discovery,
            runtime: observed_runtime,
        },
    })
}

pub(crate) fn expected_content(
    input: &HostLifecycleObservationInput,
    target_root: &Path,
) -> Result<HostLifecycleExpectedContent, &'static str> {
    validate_paths(input, target_root)?;
    let source = observe_tree(&input.marketplace_source_path)?;
    if !source.present {
        return Err("materialized marketplace source is unavailable for expected authority");
    }
    let runtime_path = input
        .marketplace_source_path
        .join("runtime/runtime-probe-bin");
    let runtime = observe_file(&runtime_path)?;
    if !runtime.present {
        return Err("materialized runtime object is unavailable for expected authority");
    }
    Ok(HostLifecycleExpectedContent {
        installed: source.digest.clone(),
        cache: source.digest,
        runtime: runtime.digest,
    })
}

fn validate_locations(
    input: &HostLifecycleObservationInput,
    target_root: &Path,
    locations: &HostSurfaceLocations,
) -> Result<(), &'static str> {
    for path in [&locations.installed, &locations.cache, &locations.runtime] {
        if !path.is_absolute()
            || path
                .components()
                .any(|component| matches!(component, std::path::Component::ParentDir))
            || !path.starts_with(target_root)
            || *path == input.marketplace_source_path
        {
            return Err("Codex host surface path is not independently confined");
        }
    }
    Ok(())
}
