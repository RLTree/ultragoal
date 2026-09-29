//! Root-owned authority boundary for supported-host distribution effects.
//!
//! This module freezes the typed authority, durable-ledger, and crate-private
//! transaction-executor interfaces. It supplies no public construction route.

mod authority;
mod executor;
mod ledger;
mod lifecycle;

pub(crate) use authority::{
    HostEffectAuthority, HostEffectDecision, HostEffectPermit, HostEffectPermitBinding,
    VerifiedHostEffectPermit,
};
pub(crate) use executor::HostEffectRecoveryHandoff;
pub(crate) use executor::{
    ConfinedHostEffectTarget, DurableHostLifecycleAdmission, HostEffectCancellation,
    HostEffectCompletion, HostEffectCompletionOutcome, HostEffectExecutionPolicy,
    NativeRetainedDescriptorProcessBackend, SupportedHostEffectExecutor,
};
pub(crate) use ledger::FileHostEffectLedger;

use super::HostCommandPlan;
use serde::Serialize;
use std::fs::{self, File, OpenOptions};

#[cfg(unix)]
use std::os::unix::fs::{FileExt, MetadataExt};

include!("max_pinned_executable_bytes.rs");
include!("host_effect_ledger_error_new.rs");
include!("same_executable_object.rs");
mod selected_codex_executable;
pub(crate) use selected_codex_executable::{SelectedCodexExecutable, resolve_codex_executable};
#[cfg(test)]
pub(crate) use selected_codex_executable::{
    SelectedCodexExecutableTestFixture, selected_test_fixture,
};
mod transaction;
mod transaction_carrier;
mod transaction_effectful;
mod transaction_failure;
mod transaction_identity;
mod transaction_observation;
mod transaction_observation_command;
mod transaction_observation_identity;
mod transaction_observation_rows;
mod transaction_observation_transition;
mod transaction_policy;
mod transaction_preparation;
mod transaction_read_only;
mod transaction_recovery;
mod transaction_recovery_carrier;
mod transaction_tree;
pub(crate) use transaction::execute_host_lifecycle_transaction;
pub(crate) use transaction_observation::HostLifecycleObservationInput;

pub(crate) fn execute_runtime_help(
    path: &std::path::Path,
    timeout: std::time::Duration,
    expected_sha256: &str,
) -> Result<(Vec<u8>, Vec<u8>), crate::distribution::DistributionError> {
    use crate::distribution::error::{DistributionErrorId, error};
    use std::os::fd::AsRawFd;
    #[cfg(unix)]
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

    let timeout_ms = u64::try_from(timeout.as_millis())
        .ok()
        .filter(|value| *value > 0 && *value <= 30_000)
        .ok_or_else(|| error(DistributionErrorId::CapabilityMismatch))?;
    let canonical = path
        .canonicalize()
        .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
    let parent = canonical
        .parent()
        .ok_or_else(|| error(DistributionErrorId::InvalidPath))?;
    #[cfg(unix)]
    let cwd = {
        let metadata = fs::symlink_metadata(parent)
            .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(error(DistributionErrorId::UnsafeObject));
        }
        let mut options = OpenOptions::new();
        options
            .read(true)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_DIRECTORY);
        let file = options
            .open(parent)
            .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
        let opened = file
            .metadata()
            .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
        if !opened.is_dir() || opened.dev() != metadata.dev() || opened.ino() != metadata.ino() {
            return Err(error(DistributionErrorId::ObjectChanged));
        }
        file
    };
    #[cfg(not(unix))]
    {
        let _ = (canonical, parent);
        return Err(error(DistributionErrorId::CapabilityMismatch));
    }

    let executable = selected_codex_executable::pin_runtime_executable(&canonical)?;
    if executable.content_sha256() != expected_sha256 {
        let _ = executable.finalize();
        return Err(error(DistributionErrorId::ProvenanceMismatch));
    }
    let capability = transaction_policy::current_capability()
        .map_err(|_| error(DistributionErrorId::CapabilityMismatch))?;
    let policy = executor::HostEffectExecutionPolicy::strict_runtime(timeout_ms)
        .map_err(|_| error(DistributionErrorId::CapabilityMismatch))?;
    let cancellation = executor::HostEffectCancellation::default();
    let command = super::HostCommand::from_untrusted_record(
        "ultragoal".to_owned(),
        vec!["--json".to_owned(), "--help".to_owned()],
        Vec::new(),
        timeout_ms,
        1,
    );
    let capture = executable.execute_runtime(
        &capability,
        &command,
        &policy,
        &cancellation,
        cwd.as_raw_fd(),
    );
    let finalized = executable.finalize();
    let capture = match capture {
        Ok(capture) => capture,
        Err(failure) => {
            let _ = finalized;
            use executor::HostEffectExecutorErrorId;
            let id = match failure.id {
                HostEffectExecutorErrorId::UnsupportedPlatform => {
                    DistributionErrorId::CapabilityMismatch
                }
                HostEffectExecutorErrorId::ProcessSpawnFailed => {
                    DistributionErrorId::ObjectUnavailable
                }
                HostEffectExecutorErrorId::OutputOverflow => DistributionErrorId::ObjectTooLarge,
                HostEffectExecutorErrorId::ExecutableMutation => DistributionErrorId::ObjectChanged,
                _ => DistributionErrorId::EffectFailed,
            };
            return Err(error(id));
        }
    };
    finalized.map_err(|_| error(DistributionErrorId::ObjectChanged))?;
    if capture.exit_code() != 0 {
        return Err(error(DistributionErrorId::EffectFailed));
    }
    Ok((capture.stdout().to_vec(), Vec::new()))
}

#[cfg(test)]
mod tests;
