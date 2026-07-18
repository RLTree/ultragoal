use super::{read, root};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

include!("source_contract/skills.rs");

include!("source_contract/validate_descriptor_bindings.rs");

include!("source_contract/six_project_agents_are_exact_and_read_only.rs");

include!("source_contract/root_wiring_request_is_exact_but_non_authoritative.rs");
