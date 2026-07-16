//! Bounded production routine definitions and immutable invocation identities.
//!
//! This module deliberately stops before effect authority. It loads one adopted
//! catalog without following links, matches every definition to an independently
//! supplied impact-graph projection, joins selected plan rows to exact transitive
//! inputs and runner observations, and emits immutable invocation descriptions.
//! It never spawns a process, creates an output directory, writes a receipt, or
//! issues a root grant.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

#[path = "catalog_contract.rs"]
mod catalog_contract;
#[path = "catalog_path_normalization.rs"]
mod catalog_path_normalization;
#[path = "catalog_validation.rs"]
mod catalog_validation;
#[path = "file_sealing.rs"]
mod file_sealing;
#[path = "invocation_binding.rs"]
mod invocation_binding;
#[path = "invocation_identity.rs"]
mod invocation_identity;
#[path = "node_selection.rs"]
mod node_selection;
#[path = "production_catalog.rs"]
mod production_catalog;
#[path = "program_path_validation.rs"]
mod program_path_validation;
#[path = "runner_identity.rs"]
mod runner_identity;
#[path = "selection_order.rs"]
mod selection_order;
#[path = "source_ancestor_capture.rs"]
mod source_ancestor_capture;

pub(crate) use catalog_contract::*;
pub(crate) use catalog_path_normalization::*;
pub(crate) use catalog_validation::*;
pub(crate) use file_sealing::*;
pub(crate) use invocation_binding::*;
pub(crate) use invocation_identity::*;
pub(crate) use node_selection::*;
pub(crate) use program_path_validation::*;
pub(crate) use runner_identity::*;
pub(crate) use selection_order::*;
pub(crate) use source_ancestor_capture::*;
