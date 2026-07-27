use super::{
    ADVISORY_SELECTION_NO_CLAIM, AdvisoryError, AdvisorySelectionDisposition,
    EngineeringAdvisorySelection,
};
use crate::digest;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const ADVISORY_ADOPTION_NO_CLAIM: &str = "An advisory adoption is root-owned planning evidence only; it cannot issue effects, accept work, or promote claims.";
const ROOT_OWNER: &str = "OWN-ULTRA-ROOT";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdvisoryAdoptionDisposition {
    Reuse,
    Extend,
    MapAsProjection,
    Reject,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineeringAdvisoryAdoption {
    pub schema_version: String,
    pub selection_id: String,
    pub candidate_id: String,
    pub context_id: String,
    pub proposal_digest: String,
    pub disposition: AdvisoryAdoptionDisposition,
    pub adopting_ultragoal_owner: String,
    pub adopted_scope: BTreeSet<String>,
    pub rejected_scope: BTreeSet<String>,
    pub verification: BTreeSet<String>,
    pub invalidation_conditions: BTreeSet<String>,
    pub claim_ceiling: String,
    pub no_claim_statement: String,
}

impl EngineeringAdvisoryAdoption {
    pub fn from_selection(
        selection: &EngineeringAdvisorySelection,
        disposition: AdvisoryAdoptionDisposition,
        adopted_scope: BTreeSet<String>,
        rejected_scope: BTreeSet<String>,
        verification: BTreeSet<String>,
    ) -> Result<Self, AdvisoryError> {
        validate_selection(selection)?;
        let adoption = Self {
            schema_version: "EngineeringAdvisoryAdoption-v1".to_owned(),
            selection_id: selection.selection_id.clone(),
            candidate_id: selection.candidate_id.clone(),
            context_id: selection.context_id.clone(),
            proposal_digest: proposal_digest(selection)?,
            disposition,
            adopting_ultragoal_owner: ROOT_OWNER.to_owned(),
            adopted_scope,
            rejected_scope,
            verification,
            invalidation_conditions: selection.invalidation_conditions.clone(),
            claim_ceiling: selection.claim_ceiling.clone(),
            no_claim_statement: ADVISORY_ADOPTION_NO_CLAIM.to_owned(),
        };
        adoption.validate_against(selection)?;
        Ok(adoption)
    }

    pub fn validate_against(
        &self,
        selection: &EngineeringAdvisorySelection,
    ) -> Result<(), AdvisoryError> {
        validate_selection(selection)?;
        if self.selection_id != selection.selection_id
            || self.candidate_id != selection.candidate_id
            || self.context_id != selection.context_id
        {
            return Err(AdvisoryError::StaleBinding);
        }
        let scope_is_valid = match self.disposition {
            AdvisoryAdoptionDisposition::Reject => {
                self.adopted_scope.is_empty() && !self.rejected_scope.is_empty()
            }
            _ => !self.adopted_scope.is_empty(),
        };
        if self.schema_version != "EngineeringAdvisoryAdoption-v1"
            || self.proposal_digest != proposal_digest(selection)?
            || self.adopting_ultragoal_owner != ROOT_OWNER
            || !scope_is_valid
            || self.verification.is_empty()
            || !self
                .invalidation_conditions
                .is_superset(&selection.invalidation_conditions)
            || self.claim_ceiling != selection.claim_ceiling
            || self.no_claim_statement != ADVISORY_ADOPTION_NO_CLAIM
        {
            return Err(AdvisoryError::InvalidContract(
                "advisory adoption authority boundary violated",
            ));
        }
        Ok(())
    }
}

fn validate_selection(selection: &EngineeringAdvisorySelection) -> Result<(), AdvisoryError> {
    if selection.schema_version != "EngineeringAdvisorySelection-v1"
        || selection.disposition != AdvisorySelectionDisposition::AdvisorySelected
        || selection.primary_lens.is_none()
        || selection.adopting_owner != ROOT_OWNER
        || !valid_digest(&selection.selection_id)
        || !valid_digest(&selection.candidate_id)
        || !valid_digest(&selection.context_id)
        || selection.claim_ceiling != "proposal_only_no_claim"
        || selection.no_claim_statement != ADVISORY_SELECTION_NO_CLAIM
    {
        return Err(AdvisoryError::StaleBinding);
    }
    Ok(())
}

fn proposal_digest(selection: &EngineeringAdvisorySelection) -> Result<String, AdvisoryError> {
    serde_json::to_vec(selection)
        .map(|bytes| digest::bytes(&bytes))
        .map_err(|_| AdvisoryError::InvalidContract("advisory selection serialization"))
}

fn valid_digest(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..].bytes().all(|byte| byte.is_ascii_hexdigit())
}
