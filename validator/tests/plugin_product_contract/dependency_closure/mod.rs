use super::{read, root};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use ultragoal::orchestration::{
    ArtifactWorkspace, CanonicalPath, EffectClass, LeaseSpec, ScopePolicy, WorkPackage,
    WorkerResultV1,
};

include!("envelope_path.rs");

include!("expected_members.rs");

include!("validate_root_metadata.rs");

include!("operations_are_exact_source_members_and_freezes_repeat_deterministically.rs");

include!("closure_temp_root_new.rs");

include!("dep_info_and_policy_reject_outside_root_and_parent_paths.rs");

include!("recovery_activation_worker_result_is_typed_but_stale_after_read_failure_guard.rs");
