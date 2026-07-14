use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

include!("supported_host_lifecycle_adapter_contract/cases.rs");

include!(
    "supported_host_lifecycle_adapter_contract/fixture_covers_the_complete_permit_and_refusal_matrix.rs"
);

include!(
    "supported_host_lifecycle_adapter_contract/darwin_contract_is_zero_reservation_zero_spawn_zero_effect_and_repeatable.rs"
);

include!(
    "supported_host_lifecycle_adapter_contract/crash_recovery_matrix_rejects_false_passes_and_never_cleans_up.rs"
);

include!(
    "supported_host_lifecycle_adapter_contract/lifecycle_candidate_has_no_public_construction_or_legacy_execution_route.rs"
);
