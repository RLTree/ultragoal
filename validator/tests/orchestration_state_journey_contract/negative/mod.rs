use super::state_fixture::*;
use crate::orchestration::product::command::*;
use crate::orchestration::product::*;
use crate::orchestration::*;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

include!("negative/running.rs");

include!(
    "negative/finding_and_root_action_membership_injection_cannot_forge_full_state_authority.rs"
);

include!("negative/interrupted_head_identity_and_current_source_are_bound_before_issuance.rs");

include!("negative/recompute_action_id.rs");
