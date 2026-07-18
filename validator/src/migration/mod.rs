//! Candidate-bound migration planning and retirement reconciliation.
//!
//! This kernel cannot mutate a registry, route, generated surface, repository,
//! or product state. It can require a root-owned authority to atomically record
//! and consume evidence-ledger state; root must separately adopt a plan and
//! broker every product effect.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub(crate) mod product;

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;

include!("max_surfaces.rs");

include!("inventory.rs");

include!("compatibility_route_new.rs");

include!("replacement/authority.rs");

include!("replacement/observation.rs");

include!("replacement/issuance.rs");

include!("replacement/consumption.rs");

include!("consumed/validation.rs");

include!("consumed/sentinel_injection_test.rs");

include!("retirement/target_digest_fragment.rs");

include!("plan/build.rs");

include!("plan/projection_verification.rs");

include!("retirement/review/issuance.rs");

include!("destructive/issuance.rs");

include!("retirement/preservation/contract.rs");

include!("retirement/decision_reconciliation.rs");

include!("replacement/ledger_currentness.rs");

include!("retirement/review/currentness.rs");

include!("destructive/currentness.rs");
