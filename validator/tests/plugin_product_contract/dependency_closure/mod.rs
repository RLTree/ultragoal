use super::{read, root};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use ultragoal::orchestration::{
    ArtifactWorkspace, CanonicalPath, EffectClass, LeaseSpec, ScopePolicy, WorkPackage,
    WorkerResultV1,
};

include!("dependency_closure/envelope_path.rs");

include!("dependency_closure/expected_members.rs");

include!("dependency_closure/validate_root_metadata.rs");

include!(
    "dependency_closure/operations_are_exact_source_members_and_freezes_repeat_deterministically.rs"
);

include!("dependency_closure/closure_temp_root_new.rs");

include!("dependency_closure/dep_info_and_policy_reject_outside_root_and_parent_paths.rs");

include!(
    "dependency_closure/recovery_activation_worker_result_is_typed_but_stale_after_read_failure_guard.rs"
);
