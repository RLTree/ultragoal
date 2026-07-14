//! Descriptor-bound durable authority for production routine mediation.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::routine_work::digest::{canonical, sha256, valid};
use crate::routine_work::{RoutineError, RoutineErrorId};

#[cfg(target_vendor = "apple")]
#[path = "supported/mod.rs"]
mod supported;
#[cfg(all(test, target_vendor = "apple"))]
#[path = "tests.rs"]
mod tests;

#[path = "authority_record.rs"]
mod authority_record;
#[path = "file_authority.rs"]
mod file_authority;

pub(crate) use authority_record::*;
pub(crate) use file_authority::*;
