use super::AdvisoryError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const ROOT_ISSUER: &str = "ultragoal-root";

/// Untrusted caller input. It cannot enter an authorized catalog until the
/// existing policy authority seals it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationModeProposal {
    pub schema_version: String,
    pub candidate_id: String,
    pub risk: String,
    pub failure_model: String,
    pub oracle: String,
    pub truth_surface: String,
    pub selected_modes: BTreeSet<String>,
    pub rejected_modes: BTreeSet<String>,
    pub required_evidence: BTreeSet<String>,
    pub claim_ceiling: BTreeMap<String, BTreeSet<String>>,
    pub invalidation_trigger: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct VerificationModeProposalInput {
    pub candidate_id: String,
    pub risk: String,
    pub failure_model: String,
    pub oracle: String,
    pub truth_surface: String,
    pub selected_modes: BTreeSet<String>,
    pub rejected_modes: BTreeSet<String>,
    pub required_evidence: BTreeSet<String>,
    pub claim_ceiling: BTreeMap<String, BTreeSet<String>>,
    pub invalidation_trigger: String,
}

impl VerificationModeProposal {
    pub fn new(input: VerificationModeProposalInput) -> Result<Self, AdvisoryError> {
        let proposal = Self {
            schema_version: "VerificationModeContract-v1".to_owned(),
            candidate_id: input.candidate_id,
            risk: input.risk,
            failure_model: input.failure_model,
            oracle: input.oracle,
            truth_surface: input.truth_surface,
            selected_modes: input.selected_modes,
            rejected_modes: input.rejected_modes,
            required_evidence: input.required_evidence,
            claim_ceiling: input.claim_ceiling,
            invalidation_trigger: input.invalidation_trigger,
        };
        proposal.validate_shape()
    }

    fn validate_shape(&self) -> Result<Self, AdvisoryError> {
        if self.schema_version != "VerificationModeContract-v1"
            || !valid_digest(&self.candidate_id)
            || self.selected_modes.is_empty()
            || self.required_evidence.is_empty()
            || self.claim_ceiling.is_empty()
            || self.selected_modes.iter().any(|mode| {
                matches!(
                    mode.to_ascii_lowercase().as_str(),
                    "tdd" | "universal-tdd" | "test-driven-development"
                )
            })
            || !self.selected_modes.is_disjoint(&self.rejected_modes)
        {
            return Err(AdvisoryError::InvalidContract(
                "verification-proposal-invalid",
            ));
        }
        for value in [
            self.risk.as_str(),
            self.failure_model.as_str(),
            self.oracle.as_str(),
            self.truth_surface.as_str(),
            self.invalidation_trigger.as_str(),
        ] {
            if value.is_empty() || value.len() > 1024 || value.chars().any(char::is_control) {
                return Err(AdvisoryError::InvalidContract(
                    "verification-contract-field-invalid",
                ));
            }
        }
        if self
            .selected_modes
            .iter()
            .chain(self.rejected_modes.iter())
            .chain(self.required_evidence.iter())
            .any(|value| {
                value.is_empty() || value.len() > 256 || value.chars().any(char::is_control)
            })
            || self.claim_ceiling.iter().any(|(claim, dimensions)| {
                claim.is_empty()
                    || dimensions.is_empty()
                    || claim.len() > 256
                    || claim.chars().any(char::is_control)
                    || dimensions.iter().any(|dimension| {
                        dimension.is_empty()
                            || dimension.len() > 256
                            || dimension.chars().any(char::is_control)
                    })
            })
        {
            return Err(AdvisoryError::InvalidContract(
                "verification-evidence-label-invalid",
            ));
        }
        Ok(self.clone())
    }
}

/// Root-sealed contract. The seal is intentionally not deserializable or
/// constructible through a public caller path.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationModeContract {
    pub schema_version: String,
    pub contract_id: String,
    pub candidate_id: String,
    pub risk: String,
    pub failure_model: String,
    pub oracle: String,
    pub truth_surface: String,
    pub selected_modes: BTreeSet<String>,
    pub rejected_modes: BTreeSet<String>,
    pub required_evidence: BTreeSet<String>,
    pub claim_ceiling: BTreeMap<String, BTreeSet<String>>,
    pub invalidation_trigger: String,
    issuer: String,
    authority_id: String,
    #[serde(skip)]
    sealed: bool,
}

impl VerificationModeContract {
    pub(crate) fn seal_from_root(
        proposal: VerificationModeProposal,
        authority_id: impl Into<String>,
    ) -> Result<Self, AdvisoryError> {
        let proposal = proposal.validate_shape()?;
        let mut contract = Self {
            schema_version: proposal.schema_version,
            contract_id: String::new(),
            candidate_id: proposal.candidate_id,
            risk: proposal.risk,
            failure_model: proposal.failure_model,
            oracle: proposal.oracle,
            truth_surface: proposal.truth_surface,
            selected_modes: proposal.selected_modes,
            rejected_modes: proposal.rejected_modes,
            required_evidence: proposal.required_evidence,
            claim_ceiling: proposal.claim_ceiling,
            invalidation_trigger: proposal.invalidation_trigger,
            issuer: ROOT_ISSUER.to_owned(),
            authority_id: authority_id.into(),
            sealed: true,
        };
        if contract.authority_id.is_empty() {
            return Err(AdvisoryError::InvalidContract(
                "verification-authority-invalid",
            ));
        }
        contract.contract_id = contract.computed_id()?;
        Ok(contract)
    }

    pub fn validate(&self) -> Result<(), AdvisoryError> {
        if !self.sealed || self.authority_id.is_empty() {
            return Err(AdvisoryError::InvalidContract(
                "verification-contract-not-root-sealed",
            ));
        }
        if self.contract_id != self.computed_id()? {
            return Err(AdvisoryError::InvalidContract(
                "verification-contract-integrity-mismatch",
            ));
        }
        VerificationModeProposal {
            schema_version: self.schema_version.clone(),
            candidate_id: self.candidate_id.clone(),
            risk: self.risk.clone(),
            failure_model: self.failure_model.clone(),
            oracle: self.oracle.clone(),
            truth_surface: self.truth_surface.clone(),
            selected_modes: self.selected_modes.clone(),
            rejected_modes: self.rejected_modes.clone(),
            required_evidence: self.required_evidence.clone(),
            claim_ceiling: self.claim_ceiling.clone(),
            invalidation_trigger: self.invalidation_trigger.clone(),
        }
        .validate_shape()?;
        Ok(())
    }

    pub fn validate_for_candidate(&self, candidate_id: &str) -> Result<(), AdvisoryError> {
        self.validate()?;
        if self.candidate_id != candidate_id {
            return Err(AdvisoryError::CandidateMismatch);
        }
        Ok(())
    }

    fn computed_id(&self) -> Result<String, AdvisoryError> {
        let bytes = serde_json::to_vec(&(
            &self.schema_version,
            &self.issuer,
            &self.authority_id,
            &self.candidate_id,
            &self.risk,
            &self.failure_model,
            &self.oracle,
            &self.truth_surface,
            &self.selected_modes,
            &self.rejected_modes,
            &self.required_evidence,
            &self.claim_ceiling,
            &self.invalidation_trigger,
        ))
        .map_err(|_| AdvisoryError::InvalidContract("verification-contract-serialization"))?;
        Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
    }
}

fn valid_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}
