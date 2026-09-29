use super::host_fixture::*;
use crate::migration::product::{
    ApplyAuthorizationAuthority, DurableMigrationStore, MigrationInputSource,
    issue_apply_authorization,
};
use std::ffi::CString;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, symlink};

include!("source_and_registry_substitution_refuse_with_zero_state_write.rs");

include!("maximum_registry_and_source_bounds_are_enforced_without_state_write.rs");
