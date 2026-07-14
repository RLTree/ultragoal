use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

include!("supported_host_effect_executor_contract/cases.rs");

include!("supported_host_effect_executor_contract/terminal_reobservation_guard_is_complete.rs");

include!("supported_host_effect_executor_contract/fixture_contract/mod.rs");

include!(
    "supported_host_effect_executor_contract/executor_orders_command_publication_ledger_and_acknowledgement.rs"
);

include!(
    "supported_host_effect_executor_contract/post_reservation_reobservation_requires_coherent_head_and_recovering_permit_before_exact_classif.rs"
);

include!(
    "supported_host_effect_executor_contract/process_backend_is_descriptor_bound_empty_environment_contained_and_darwin_closed.rs"
);
