use super::SelectedCodexExecutable;
use super::transaction_observation::HostLifecycleObservationInput;
use crate::distribution::HostCommand;
use crate::distribution::host_effect::executor::{
    HostEffectCancellation, HostEffectExecutionPolicy, NativeRetainedDescriptorProcessBackend,
    execute_bounded_observation,
};
use crate::distribution::host_effect::lifecycle::DescriptorExecutionCapability;
use serde_json::Value;
use std::os::fd::RawFd;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

pub(super) fn run_json(
    backend: &mut NativeRetainedDescriptorProcessBackend,
    capability: &DescriptorExecutionCapability,
    executable: &SelectedCodexExecutable,
    policy: &HostEffectExecutionPolicy,
    cancellation: &HostEffectCancellation,
    cwd: RawFd,
    command: HostCommand,
) -> Result<Value, &'static str> {
    let capture = execute_bounded_observation(
        backend,
        capability,
        executable,
        &command,
        policy,
        cancellation,
        cwd,
    )
    .map_err(|_| "bounded Codex observation command failed")?;
    if capture.exit_code() != 0 {
        return Err("Codex observation command returned failure");
    }
    serde_json::from_slice(capture.stdout()).map_err(|_| "Codex observation JSON invalid")
}

pub(super) fn observation_command(args: &[&str], environment: &[(String, String)]) -> HostCommand {
    HostCommand::from_untrusted_record(
        "codex".to_owned(),
        args.iter().map(|value| (*value).to_owned()).collect(),
        environment.to_vec(),
        30_000,
        1,
    )
}

pub(super) fn validate_input(
    input: &HostLifecycleObservationInput,
    target_root: &Path,
    cwd: RawFd,
) -> Result<(), &'static str> {
    validate_paths(input, target_root)?;
    // SAFETY: cwd is the live descriptor supplied by the confined target and stat is writable.
    let mut stat = unsafe { std::mem::zeroed::<libc::stat>() };
    // SAFETY: fstat only reads the descriptor identity into the writable stat buffer.
    if unsafe { libc::fstat(cwd, &mut stat) } != 0 {
        return Err("host observation cwd is not the bound root");
    }
    let root =
        std::fs::symlink_metadata(target_root).map_err(|_| "host observation root unavailable")?;
    if root.file_type().is_symlink() || !root.is_dir() {
        return Err("host observation root is not a directory");
    }
    if stat.st_dev as u64 != root.dev() || stat.st_ino as u64 != root.ino() {
        return Err("host observation cwd is not the bound root");
    }
    Ok(())
}

pub(super) fn validate_paths(
    input: &HostLifecycleObservationInput,
    target_root: &Path,
) -> Result<(), &'static str> {
    let root =
        std::fs::symlink_metadata(target_root).map_err(|_| "host observation root unavailable")?;
    if root.file_type().is_symlink() || !root.is_dir() {
        return Err("host observation root is not a directory");
    }
    for path in [
        &input.marketplace_source_path,
        &input.marketplace_source_root,
    ] {
        if !path.is_absolute()
            || path
                .components()
                .any(|component| matches!(component, std::path::Component::ParentDir))
            || !path.starts_with(target_root)
        {
            return Err("host observation path is not confined");
        }
    }
    Ok(())
}
