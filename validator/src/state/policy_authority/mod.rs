use super::catalog::{ClaimSpec, DependencyActionCatalog, DependencyActionSpec};
use super::product_state::StateError;
use crate::context::LiveContext;
use crate::inventory::AuthorityCatalog;
use serde::Serialize;
use std::collections::BTreeSet;

#[path = "policy_permit.rs"]
mod policy_permit;
#[path = "policy_validation.rs"]
mod policy_validation;

pub(crate) use policy_permit::*;
pub(crate) use policy_validation::*;
