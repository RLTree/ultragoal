use super::fixture::*;
use crate::context::EffectClass;
use crate::state::catalog::{ClaimSpec, DependencyActionCatalog, InventoryPolicy};
use crate::state::engine::derive_bound;
use crate::state::policy_authority::{self, PolicyAuthority};
use crate::state::product_state::{AuthorityRequirement, StateError};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

#[path = "adopted_claim_cases.rs"]
mod adopted_claim_cases;
#[path = "permit_mutation_cases.rs"]
mod permit_mutation_cases;

pub(crate) use adopted_claim_cases::*;
pub(crate) use permit_mutation_cases::*;
