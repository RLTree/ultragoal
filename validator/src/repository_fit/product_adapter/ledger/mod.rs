//! Durable repository-fit production reservation and terminal ledger.
//!
//! This deliberately owns repository-fit records instead of reusing the
//! distribution host-effect authority model. It borrows the accepted storage
//! properties: descriptor binding, owner-only objects, a process-shared lock,
//! an authenticated hash chain, atomic publication, and directory fsync.

use serde::{Deserialize, Serialize};

use super::root_permit::ManagedAncestorContract;
use super::{AdapterErrorId, FitAdapterError, adapter_error};

#[cfg(target_vendor = "apple")]
#[path = "supported/mod.rs"]
mod supported;

#[path = "effect_ownership.rs"]
mod effect_ownership;
#[path = "ledger_failure.rs"]
mod ledger_failure;
#[path = "recovery_record.rs"]
mod recovery_record;

#[cfg(test)]
pub(crate) use effect_ownership::{
    before_atomic_publish_for_test, before_existing_open_for_test, before_lock_acquire_for_test,
};
pub(crate) use ledger_failure::*;
pub(crate) use recovery_record::*;
