use super::definition::{ADOPTED_CLAIM_REGISTRY_SHA256, ClaimDefinition, ClaimDefinitions};
#[cfg(test)]
use super::evidence::{Actor, ActorRole};
pub use super::false_pass_receipt::{SemanticControlModel, SemanticControlObservation};
use crate::context::LiveContext;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

#[path = "control_catalog.rs"]
mod control_catalog;
#[path = "control_plan.rs"]
mod control_plan;
#[path = "negative_control_authority.rs"]
mod negative_control_authority;
#[path = "semantic_control_authority.rs"]
mod semantic_control_authority;

pub(crate) use control_catalog::*;
pub(crate) use control_plan::*;
pub(crate) use negative_control_authority::*;
pub use semantic_control_authority::LocalNegativeControlAuthority;
#[cfg(test)]
pub(crate) use semantic_control_authority::MODEL_SEQUENCE;
pub(crate) use semantic_control_authority::{
    CONTROL_CATALOG_VERSION, MODEL_IMPLEMENTATION_VERSION, MODEL_MAX_AGE_MS, MODEL_METHOD,
    MODEL_OBSERVATION_METHOD, ModelAuthority, PRIVATE_TRANSPORT_UNAVAILABLE,
};
