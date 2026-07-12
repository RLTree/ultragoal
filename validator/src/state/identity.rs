use super::catalog::RuntimeMetadata;
use super::ceiling::ClaimCeiling;
use super::types::{Finding, NextAction, ProductGoalState, Repair, StateError};
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Serialize)]
pub(crate) struct StateIdentity<'a> {
    pub schema_version: &'static str,
    pub context_id: &'a str,
    pub authority_catalog_id: &'a str,
    pub dependency_action_catalog_id: &'a str,
    pub product_goal: ProductGoalState,
    pub runtime_metadata: &'a RuntimeMetadata,
    pub findings: &'a [Finding],
    pub repairs: &'a [Repair],
    pub claim_ceilings: &'a [ClaimCeiling],
    pub next_action: &'a NextAction,
}

pub(crate) fn state_id(identity: StateIdentity<'_>) -> Result<String, StateError> {
    let bytes = serde_json::to_vec(&identity)
        .map_err(|error| StateError::Serialization(error.to_string()))?;
    if bytes.len() > super::limits::MAX_PROJECTION_BYTES {
        return Err(StateError::ResourceLimit(
            "product state identity bytes".to_owned(),
        ));
    }
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}
