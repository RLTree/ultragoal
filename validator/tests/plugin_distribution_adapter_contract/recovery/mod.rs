use crate::lifecycle_fixture::{Fixture, installed, lifecycle, request};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::{Arc, Barrier, Mutex};
use ultragoal::plugin_product::lifecycle::{
    ApplyDisposition, LifecycleError, LifecycleIntent, LifecycleState,
    recovery_token as issue_sibling_recovery_token,
};

include!("partial_cache_failure_rolls_back_only_the_completed_install_prefix.rs");

include!("same_root_sibling_operation_cannot_issue_or_use_owner_token.rs");
