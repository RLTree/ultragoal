//! Root-owned authority boundary for supported-host distribution effects.
//!
//! This module freezes the typed authority, durable-ledger, and crate-private
//! transaction-executor interfaces. It deliberately supplies no public
//! construction route and no Darwin external-process adapter. Platform adapters
//! remain separate reviewed work, and a platform that cannot execute a retained
//! descriptor must fail as unsupported before reservation, spawn, or host
//! mutation.

mod authority;
mod executor;
mod ledger;
mod lifecycle;

pub(crate) use authority::{
    HostEffectAuthority, HostEffectDecision, HostEffectPermit, HostEffectPermitBinding,
    VerifiedHostEffectPermit,
};
pub(crate) use executor::{
    DurableHostLifecycleAdmission, HostEffectCompletion, HostEffectCompletionOutcome,
};
pub(crate) use ledger::FileHostEffectLedger;

use super::HostCommandPlan;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::{FileExt, MetadataExt, OpenOptionsExt};

include!("max_pinned_executable_bytes.rs");

include!("host_effect_ledger_error_new.rs");

include!("same_executable_object.rs");

#[cfg(test)]
mod tests;
