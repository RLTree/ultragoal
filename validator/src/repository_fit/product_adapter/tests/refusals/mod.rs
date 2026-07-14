use super::scenario::{Fixture, assert_zero_write, snapshot};
use crate::repository_fit::product_adapter::{
    AdapterErrorId, inspect_target, plan_target, prepare_apply_request,
};
use crate::repository_fit::{CanonicalPath, ExpectedContent, FitEffects, FitErrorId, digest};
use serde_json::Value;
use std::collections::BTreeMap;
use std::ffi::CString;
use std::fs::{self, OpenOptions};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::symlink;
use std::sync::{Arc, Barrier};

#[path = "digest_fixture.rs"]
mod digest_fixture;
#[path = "leaf_path_swap_and_stale_expected_digest_preserve_foreign_bytes.rs"]
mod leaf_path_swap_and_stale_expected_digest_preserve_foreign_bytes;
#[path = "removal_cleanup_quarantines_a_substituted_temp_without_deleting_it.rs"]
mod removal_cleanup_quarantines_a_substituted_temp_without_deleting_it;

pub(crate) use digest_fixture::*;
pub(crate) use leaf_path_swap_and_stale_expected_digest_preserve_foreign_bytes::*;
pub(crate) use removal_cleanup_quarantines_a_substituted_temp_without_deleting_it::*;
