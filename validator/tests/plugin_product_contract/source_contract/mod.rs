use super::{read, root};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

include!("skills.rs");

include!("validate_descriptor_bindings.rs");

include!("six_project_agents_are_exact_and_read_only.rs");

include!("root_wiring_request_is_exact_but_non_authoritative.rs");
