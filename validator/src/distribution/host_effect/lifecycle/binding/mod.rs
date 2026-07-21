use super::{SupportedHostLifecycleError, SupportedHostLifecycleErrorId, lifecycle_error};
use crate::distribution::{
    Capability, HostCapabilityDeclaration, HostCapabilityState, HostCommandPlan, JourneyBinding,
    PackageIdentity,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::Metadata;
use std::path::Path;

use super::super::{
    HostEffectDecision, HostEffectLedgerHead, HostEffectPermitBinding, SelectedCodexExecutable,
};

include!("session_nonce_bytes.rs");

include!("accepted_intent.rs");

include!("accepted/personal_scope.rs");

include!("observed_target_identity_new.rs");

include!("accepted/effect.rs");

include!("accepted/derived_binding.rs");

include!("argv_sha256.rs");
