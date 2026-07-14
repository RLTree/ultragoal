use super::catalog::{DependencyActionCatalog, DependencyStatus};
use super::ceiling::ClaimCeiling;
use super::findings::{
    authority_name, contradiction_repair, dependency_code, dependency_severity, finding,
    policy_repair,
};
use super::product_state::{
    CeilingReduction, Finding, FindingSeverity, FindingSource, NextActionKind, ProductGoalState,
    ProductState, Scope, StateError,
};
use super::snapshot::BoundInputs;
use super::{identity, next, observations, policy_authority, reduce};
use crate::context::LiveContext;
use crate::inventory::{AuthorityCatalog, FindingSeverity as InventorySeverity};
use std::collections::{BTreeMap, BTreeSet};

#[path = "inventory_reduction.rs"]
mod inventory_reduction;
#[path = "state_derivation.rs"]
mod state_derivation;

pub(crate) use inventory_reduction::*;
pub use state_derivation::StateEngine;
pub(crate) use state_derivation::{all_reductions, derive_bound, initial_ceilings, policy_finding};
