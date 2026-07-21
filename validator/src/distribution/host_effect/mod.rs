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
pub(crate) use executor::{
    ConfinedHostEffectTarget, DurableHostLifecycleAdmission, HostEffectCancellation,
    HostEffectCompletion, HostEffectCompletionOutcome, HostEffectExecutionPolicy,
    NativeRetainedDescriptorProcessBackend, SupportedHostEffectExecutor,
};
pub(crate) use ledger::FileHostEffectLedger;

use super::HostCommandPlan;
use serde::Serialize;
use std::fs::{self, File};

#[cfg(test)]
use std::fs::OpenOptions;
#[cfg(test)]
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::{FileExt, MetadataExt};

include!("max_pinned_executable_bytes.rs");
include!("host_effect_ledger_error_new.rs");
include!("same_executable_object.rs");
mod selected_codex_executable;
pub(crate) use selected_codex_executable::{resolve_codex_executable, SelectedCodexExecutable};
mod transaction;
mod transaction_identity;
mod transaction_observation;
mod transaction_observation_command;
mod transaction_observation_identity;
mod transaction_observation_transition;
mod transaction_policy;
mod transaction_read_only;
mod transaction_recovery;
mod transaction_tree;
pub(crate) use transaction::execute_host_lifecycle_transaction;
pub(crate) use transaction_observation::HostLifecycleObservationInput;

#[cfg(test)]
mod tests;
