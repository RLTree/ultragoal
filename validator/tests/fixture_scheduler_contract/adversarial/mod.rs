use crate::fixture_scheduler::*;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

include!("root_expected.rs");

include!("recovery/unrelated_root.rs");

include!("recovery/child_replacement.rs");

include!("child/swap_between_identity_check_and_atomic_capture_fails_closed.rs");

include!("child/directory_swap_after_capture_validation_retains_both_directories.rs");
