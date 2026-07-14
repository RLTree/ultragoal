use super::binding::BoundPackage;
use super::error::{AdapterError, AdapterErrorId};
use super::mutation::{Mutation, Surface};
use super::observation::{CACHE_PATH, INSTALLED_PATH, observe, verify_surface};
use crate::distribution::{
    ConfinedRoot, ExpectedPrior, InstallPlan, InstallScope, RollbackInstallError, ScopedFile,
    ScopedInstall, install, rollback_install,
};
use crate::plugin_product::lifecycle::{LifecycleEffect, LifecycleEffectAdapter, LifecycleState};
use sha2::{Digest, Sha256};

include!("state.rs");

include!("binding.rs");

include!("restoration.rs");

#[cfg(test)]
include!("restoration_tests.rs");
