use super::insert_prefix_free_path;
use super::plan::{PackageEntry, PackagePlan};
use super::spec::{ENTRY_LIMIT, PACKAGE_LIMIT, PackageRole};
use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::reader::{sha256, validate_relative_path};
use crate::distribution::spec::digest;
use crate::plugin_manifest::Version;
use serde::Serialize;
use std::collections::BTreeSet;

include!("magic.rs");

include!("inventory.rs");
