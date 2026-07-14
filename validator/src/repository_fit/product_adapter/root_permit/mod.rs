//! Root-mediated one-use repository-fit apply authority.
//!
//! This module is intentionally internal. Permit issuance and public apply
//! dispatch remain root-owned and are not activated by this increment.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[cfg(unix)]
use std::ffi::{CStr, CString, OsStr};
use std::fmt::{Debug, Formatter};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[cfg(test)]
use std::cell::RefCell;
#[cfg(test)]
use std::sync::Mutex;

#[cfg(unix)]
use std::mem::MaybeUninit;
#[cfg(unix)]
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd};
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt};

use crate::context::{BuildRequest, LiveContext};
#[cfg(unix)]
use crate::repository_fit::local::root_binding_from_canonical_path;
use crate::repository_fit::{
    CanonicalPath, ExpectedContent, FitEffects, FitError, FitErrorId, FitReader, FitVerification,
    apply, digest, rollback, valid_digest, verify,
};

#[cfg(unix)]
use crate::repository_fit::LocalEffects;

use super::protocol::{ApplyRequestSeal, OpaqueFitApplyRequest, revalidate_apply_request};
use super::{AdapterErrorId, FitAdapterError, adapter_error};

#[path = "authorization_test_hooks.rs"]
mod authorization_test_hooks;
#[path = "change_version.rs"]
mod change_version;
#[path = "desired_target_validation.rs"]
mod desired_target_validation;
#[path = "effect_confinement.rs"]
mod effect_confinement;
#[path = "effect_preflight.rs"]
mod effect_preflight;
#[path = "mutation_lease.rs"]
mod mutation_lease;
#[path = "object_metadata.rs"]
mod object_metadata;
#[path = "permit_activation.rs"]
mod permit_activation;
#[path = "permit_identity.rs"]
mod permit_identity;
#[path = "permit_scope.rs"]
mod permit_scope;
#[path = "permitted_application.rs"]
mod permitted_application;
#[path = "prior_state_reconciliation.rs"]
mod prior_state_reconciliation;
#[path = "protected_descriptor_walk.rs"]
mod protected_descriptor_walk;
#[path = "protected_path_capture.rs"]
mod protected_path_capture;
#[path = "protected_state_capture.rs"]
mod protected_state_capture;
#[path = "recovery_leaf.rs"]
mod recovery_leaf;
#[path = "recovery_observation.rs"]
mod recovery_observation;
#[path = "target_attachment.rs"]
mod target_attachment;
#[path = "versioned_object.rs"]
mod versioned_object;

pub(crate) use change_version::*;
pub(crate) use desired_target_validation::*;
pub(crate) use effect_confinement::*;
pub(crate) use effect_preflight::*;
pub(crate) use mutation_lease::*;
pub(crate) use object_metadata::*;
pub(crate) use permit_activation::*;
pub(crate) use permit_identity::*;
pub(crate) use permit_scope::*;
pub(crate) use permitted_application::*;
pub(crate) use prior_state_reconciliation::*;
pub(crate) use protected_descriptor_walk::*;
pub(crate) use protected_path_capture::*;
pub(crate) use protected_state_capture::*;
pub(crate) use recovery_leaf::*;
pub(crate) use recovery_observation::*;
pub(crate) use target_attachment::*;
pub(crate) use versioned_object::*;

#[cfg(test)]
pub(crate) use authorization_test_hooks::*;
