//! Candidate-bound migration planning and retirement reconciliation.
//!
//! This kernel cannot mutate a registry, route, generated surface, repository,
//! or product state. It can require a root-owned authority to atomically record
//! and consume evidence-ledger state; root must separately adopt a plan and
//! broker every product effect.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(test)]
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;

pub(crate) mod product;

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;

include!("max_surfaces.rs");

include!("inventory.rs");

include!("input_contract.rs");

#[cfg(test)]
include!("compatibility_route_new.rs");

#[cfg(test)]
include!("replacement/authority.rs");

#[cfg(test)]
include!("replacement/observation.rs");

#[cfg(test)]
include!("replacement/issuance.rs");

#[cfg(test)]
include!("replacement/consumption.rs");

#[cfg(test)]
include!("consumed/validation.rs");

#[cfg(test)]
include!("consumed/sentinel_injection_test.rs");

#[cfg(test)]
include!("retirement/target_digest_fragment.rs");

#[cfg(test)]
include!("plan/build.rs");

#[cfg(test)]
include!("plan/projection_verification.rs");

#[cfg(test)]
include!("retirement/review/issuance.rs");

#[cfg(test)]
include!("destructive/issuance.rs");

#[cfg(test)]
include!("retirement/preservation/contract.rs");

#[cfg(test)]
include!("retirement/decision_reconciliation.rs");

#[cfg(test)]
include!("replacement/ledger_currentness.rs");

#[cfg(test)]
include!("retirement/review/currentness.rs");

#[cfg(test)]
include!("destructive/currentness.rs");
