use super::selection_response::selection_identity;
use super::{
    ADVISORY_SELECTION_NO_CLAIM, ADVISORY_SELECTION_SCHEMA, ADVISORY_SELECTOR_VERSION,
    AdvisoryError, AdvisorySelectionDisposition, EngineeringAdvisorySelection,
};
use crate::digest;
use crate::plugin_product::engineering_advisory::{
    AGENTIC_PACK_SET_DIGEST, AgenticCandidateBinding, exact_agentic_candidate_binding,
    qualify_skill,
};
use crate::plugin_product::skill_catalog::{
    agentic_stage_profile_contains_qualified_skill, is_agentic_stage_profile_digest,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const ADVISORY_ADOPTION_NO_CLAIM: &str = "An advisory adoption is root-owned planning evidence only; it cannot issue effects, accept work, or promote claims.";
pub const ADVISORY_ADOPTION_SCHEMA: &str = "EngineeringAdvisoryAdoption-v2";
pub const ADVISORY_ADOPTION_PROPOSAL_DOMAIN: &str = "EngineeringAdvisoryAdoptionProposal-v2";
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
pub struct EngineeringAdvisoryAdoptionBinding {
    pub schema_version: String,
    pub selection_id: String,
    pub candidate_id: String,
    pub context_id: String,
    pub pack_set_digest: String,
    pub profile_digest: String,
    pub selector_version: String,
    pub agentic_candidate: AgenticCandidateBinding,
}

impl EngineeringAdvisoryAdoptionBinding {
    pub fn exact(
        selection_id: impl Into<String>,
        candidate_id: impl Into<String>,
        context_id: impl Into<String>,
        profile_digest: impl Into<String>,
    ) -> Result<Self, AdvisoryError> {
        let binding = Self {
            schema_version: "EngineeringAdvisoryAdoptionBinding-v1".to_owned(),
            selection_id: selection_id.into(),
            candidate_id: candidate_id.into(),
            context_id: context_id.into(),
            pack_set_digest: AGENTIC_PACK_SET_DIGEST.to_owned(),
            profile_digest: profile_digest.into(),
            selector_version: ADVISORY_SELECTOR_VERSION.to_owned(),
            agentic_candidate: exact_agentic_candidate_binding(),
        };
        binding.validate()?;
        Ok(binding)
    }

    fn validate(&self) -> Result<(), AdvisoryError> {
        if self.schema_version != "EngineeringAdvisoryAdoptionBinding-v1"
            || !valid_digest(&self.selection_id)
            || !valid_digest(&self.candidate_id)
            || !valid_digest(&self.context_id)
            || self.pack_set_digest != AGENTIC_PACK_SET_DIGEST
            || !is_agentic_stage_profile_digest(&self.profile_digest)
            || self.selector_version != ADVISORY_SELECTOR_VERSION
            || self.agentic_candidate != exact_agentic_candidate_binding()
        {
            return Err(AdvisoryError::StaleBinding);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineeringAdvisoryAdoption {
    pub schema_version: String,
    pub selection_id: String,
    pub candidate_id: String,
    pub context_id: String,
    pub pack_set_digest: String,
    pub profile_digest: String,
    pub selector_version: String,
    pub agentic_candidate: AgenticCandidateBinding,
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
        expected: &EngineeringAdvisoryAdoptionBinding,
        disposition: AdvisoryAdoptionDisposition,
        adopted_scope: BTreeSet<String>,
        rejected_scope: BTreeSet<String>,
        verification: BTreeSet<String>,
    ) -> Result<Self, AdvisoryError> {
        validate_selection(selection, expected)?;
        let adoption = Self {
            schema_version: ADVISORY_ADOPTION_SCHEMA.to_owned(),
            selection_id: selection.selection_id.clone(),
            candidate_id: selection.candidate_id.clone(),
            context_id: selection.context_id.clone(),
            pack_set_digest: selection.pack_set_digest.clone(),
            profile_digest: selection.profile_digest.clone(),
            selector_version: selection.selector_version.clone(),
            agentic_candidate: selection
                .agentic_candidate
                .clone()
                .ok_or(AdvisoryError::StaleBinding)?,
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
        adoption.validate_against(selection, expected)?;
        Ok(adoption)
    }

    pub fn validate_against(
        &self,
        selection: &EngineeringAdvisorySelection,
        expected: &EngineeringAdvisoryAdoptionBinding,
    ) -> Result<(), AdvisoryError> {
        validate_selection(selection, expected)?;
        if self.selection_id != selection.selection_id
            || self.candidate_id != selection.candidate_id
            || self.context_id != selection.context_id
            || self.pack_set_digest != selection.pack_set_digest
            || self.profile_digest != selection.profile_digest
            || self.selector_version != selection.selector_version
            || selection.agentic_candidate.as_ref() != Some(&self.agentic_candidate)
            || self.candidate_id != expected.candidate_id
            || self.context_id != expected.context_id
            || self.pack_set_digest != expected.pack_set_digest
            || self.profile_digest != expected.profile_digest
            || self.selector_version != expected.selector_version
            || self.agentic_candidate != expected.agentic_candidate
            || self.selection_id != expected.selection_id
        {
            return Err(AdvisoryError::StaleBinding);
        }
        let scope_is_valid = match self.disposition {
            AdvisoryAdoptionDisposition::Reject => {
                self.adopted_scope.is_empty() && !self.rejected_scope.is_empty()
            }
            _ => !self.adopted_scope.is_empty(),
        };
        if self.schema_version != ADVISORY_ADOPTION_SCHEMA
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

fn validate_selection(
    selection: &EngineeringAdvisorySelection,
    expected: &EngineeringAdvisoryAdoptionBinding,
) -> Result<(), AdvisoryError> {
    expected.validate()?;
    let qualified_primary_matches = selection
        .primary_lens
        .is_some_and(|lens| selection.qualified_primary_skill == qualify_skill(lens.skill_name()));
    let qualified_supporting = selection
        .supporting_lenses
        .iter()
        .filter_map(|lens| qualify_skill(lens.skill_name()))
        .collect::<Vec<_>>();
    let routes_match_profile = selection
        .qualified_primary_skill
        .as_deref()
        .is_some_and(|route| {
            agentic_stage_profile_contains_qualified_skill(&selection.profile_digest, route)
        })
        && selection.qualified_supporting_skills.iter().all(|route| {
            agentic_stage_profile_contains_qualified_skill(&selection.profile_digest, route)
        });
    let unique_supporting_lenses = selection
        .supporting_lenses
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let expected_selection_id = selection_identity(selection);
    if selection.schema_version != ADVISORY_SELECTION_SCHEMA
        || selection.disposition != AdvisorySelectionDisposition::AdvisorySelected
        || !qualified_primary_matches
        || selection.qualified_supporting_skills != qualified_supporting
        || !routes_match_profile
        || unique_supporting_lenses.len() != selection.supporting_lenses.len()
        || selection
            .primary_lens
            .is_some_and(|primary| unique_supporting_lenses.contains(&primary))
        || selection.pack_set_digest != AGENTIC_PACK_SET_DIGEST
        || selection.selection_id != expected.selection_id
        || selection.candidate_id != expected.candidate_id
        || selection.context_id != expected.context_id
        || selection.pack_set_digest != expected.pack_set_digest
        || selection.profile_digest != expected.profile_digest
        || selection.selector_version != expected.selector_version
        || selection.agentic_candidate.as_ref() != Some(&expected.agentic_candidate)
        || !is_agentic_stage_profile_digest(&selection.profile_digest)
        || selection.selector_version != ADVISORY_SELECTOR_VERSION
        || selection.agentic_candidate.as_ref() != Some(&exact_agentic_candidate_binding())
        || selection.adopting_owner != ROOT_OWNER
        || selection.selection_id != expected_selection_id
        || !valid_digest(&selection.selection_id)
        || !valid_digest(&selection.input_fingerprint)
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
    serde_json::to_vec(&(ADVISORY_ADOPTION_PROPOSAL_DOMAIN, selection))
        .map(|bytes| digest::bytes(&bytes))
        .map_err(|_| AdvisoryError::InvalidContract("advisory selection serialization"))
}

fn valid_digest(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..].bytes().all(|byte| byte.is_ascii_hexdigit())
}
