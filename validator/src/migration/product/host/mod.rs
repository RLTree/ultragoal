//! Darwin production adapters for the accepted migration product protocol.
//!
//! The adapters are crate-private and require a separately preprovisioned,
//! owner-only state root. They do not adopt registry rows or expose a public
//! command. Repository bytes are descriptor-anchored and remain physically
//! untouched; only the fixed host ledger records semantic route state.

mod authority;
mod effects;
mod filesystem;
mod source;
mod store;

#[cfg(test)]
mod test_host_provisioning;

use self::authority::{DarwinMigrationAuthority, DarwinMigrationAuthorityFactory};
use self::effects::DarwinMigrationEffects;
use self::filesystem::{AnchoredDirectory, FileIdentity, ProcessLock};
use self::source::DarwinMigrationSource;
use self::store::DarwinMigrationStore;
use super::super::digest as digest_bytes;
use super::{ProductMigrationError, ProductMigrationPlan};
use crate::migration::MigrationInventory;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;
use std::sync::{Arc, Mutex};

#[cfg(test)]
pub(crate) use self::authority::DarwinMigrationAuthority as DarwinApplyAuthorizationAuthority;
#[cfg(test)]
pub(crate) use self::effects::DarwinMigrationEffects as DarwinConfinedMigrationEffect;
#[cfg(test)]
pub(crate) use self::source::DarwinMigrationSource as DarwinMigrationInputSource;
#[cfg(test)]
pub(crate) use self::store::DarwinMigrationStore as DarwinDurableMigrationStore;

#[cfg(test)]
pub(crate) use self::test_host_provisioning::provision_darwin_migration_host_for_test;

include!("config_name.rs");

include!("host_context_open.rs");

include!("trusted_time.rs");
