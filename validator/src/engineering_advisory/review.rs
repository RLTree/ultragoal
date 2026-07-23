use super::AdvisoryError;
use crate::orchestration::{Binding, ReviewDecision, ReviewRecord};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewMaterialityOutput {
    pub binding: Binding,
    pub review_id: String,
    pub claim_ceiling: BTreeMap<String, BTreeSet<String>>,
    pub unverifiable_claims: BTreeSet<String>,
    pub rerun_command_id: String,
    pub material: bool,
}

pub type MaterialityOutput = ReviewMaterialityOutput;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewVerdict {
    pub schema_version: String,
    pub binding: Binding,
    pub review_id: String,
    pub reviewer: String,
    pub worker: String,
    pub decision: ReviewDecision,
    pub material: bool,
    pub independent: bool,
    pub unverifiable_claims: BTreeSet<String>,
    pub claim_ceiling: BTreeMap<String, BTreeSet<String>>,
    pub rerun_command_id: String,
    pub reviewer_can_promote: bool,
    pub no_claim_statement: String,
}

pub const REVIEW_VERDICT_NO_CLAIM: &str =
    "Review advice cannot accept, integrate, promote, or raise a claim ceiling.";

impl ReviewVerdict {
    pub fn from_review(
        review: &ReviewRecord,
        materiality: &ReviewMaterialityOutput,
    ) -> Result<Self, AdvisoryError> {
        review
            .validate()
            .map_err(|_| AdvisoryError::InvalidReview("review record is invalid"))?;
        let review_id = review
            .review_id()
            .map_err(|_| AdvisoryError::InvalidReview("review identity is invalid"))?;
        if materiality.review_id != review_id || materiality.binding != review.binding {
            return Err(AdvisoryError::StaleBinding);
        }
        if !review
            .reproduced_commands
            .contains(&materiality.rerun_command_id)
        {
            return Err(AdvisoryError::InvalidReview(
                "review rerun is not reproduced",
            ));
        }
        if materiality.claim_ceiling.iter().any(|(claim, dimensions)| {
            claim.is_empty()
                || dimensions.is_empty()
                || claim.contains("complete")
                || dimensions.iter().any(|dimension| {
                    matches!(
                        dimension.as_str(),
                        "material_signoff"
                            | "major_root_integration"
                            | "product"
                            | "readiness"
                            | "release"
                            | "completion"
                    )
                })
        }) {
            return Err(AdvisoryError::ClaimPromotion);
        }
        let decision = if !materiality.unverifiable_claims.is_empty() {
            ReviewDecision::Rework
        } else if !materiality.material {
            ReviewDecision::Rework
        } else {
            review.decision
        };
        Ok(Self {
            schema_version: "ReviewVerdict-v1".to_owned(),
            binding: review.binding.clone(),
            review_id,
            reviewer: review.reviewer.clone(),
            worker: review.worker.clone(),
            decision,
            material: materiality.material,
            independent: review.reviewer != review.worker,
            unverifiable_claims: materiality.unverifiable_claims.clone(),
            claim_ceiling: materiality.claim_ceiling.clone(),
            rerun_command_id: materiality.rerun_command_id.clone(),
            reviewer_can_promote: false,
            no_claim_statement: REVIEW_VERDICT_NO_CLAIM.to_owned(),
        })
    }

    pub fn validate(&self) -> Result<(), AdvisoryError> {
        if self.schema_version != "ReviewVerdict-v1"
            || !self.independent
            || self.reviewer_can_promote
            || self.no_claim_statement != REVIEW_VERDICT_NO_CLAIM
            || self.reviewer == self.worker
            || self.rerun_command_id.is_empty()
        {
            return Err(AdvisoryError::InvalidReview(
                "review verdict authority boundary violated",
            ));
        }
        if !self.unverifiable_claims.is_empty() && self.decision == ReviewDecision::Pass {
            return Err(AdvisoryError::InvalidReview(
                "unverifiable review cannot pass",
            ));
        }
        Ok(())
    }
}
