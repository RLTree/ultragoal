mod model;
mod reader;

// The v1/v2 brief parser is retained only for frozen compatibility fixtures.
// Current product inspection and state derivation do not load or rank it.
#[cfg(test)]
mod normalize;
#[cfg(test)]
mod parser;
#[cfg(test)]
mod ranking;
#[cfg(test)]
mod validation;

#[cfg(test)]
mod tests;

use model::{CandidateBinding, CurrentAuthorityBinding, CurrentAuthorityFacts};
use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum InceptionError {
    Context,
    ContractBindingInvalid,
    UnsafeInput,
    Code(&'static str),
}

impl InceptionError {
    fn context(_: impl std::fmt::Display) -> Self {
        Self::Context
    }

    pub(crate) fn cause(&self) -> &'static str {
        match self {
            Self::Context => "the live context changed during current-authority inspection",
            Self::ContractBindingInvalid => {
                "the current goal, product contract, or active plan binding is invalid"
            }
            Self::UnsafeInput => "a current authority input is not a safe regular file",
            Self::Code(code) => code,
        }
    }
}

impl From<&'static str> for InceptionError {
    fn from(code: &'static str) -> Self {
        Self::Code(code)
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MissingProjection {
    pub(crate) schema_version: &'static str,
    pub(crate) status: &'static str,
    pub(crate) claim_ceiling: &'static str,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CurrentProjection {
    pub(crate) schema_version: &'static str,
    pub(crate) status: &'static str,
    pub(crate) context_id: String,
    pub(crate) candidate: CandidateBinding,
    pub(crate) current_authority: CurrentAuthorityBinding,
    pub(crate) claim_ids: Vec<String>,
    pub(crate) ranking_eligible: bool,
    pub(crate) claim_ceiling: &'static str,
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub(crate) enum Projection {
    #[allow(dead_code)]
    Missing(MissingProjection),
    Current(CurrentProjection),
}

impl Projection {
    pub(crate) fn to_json(&self) -> Result<Vec<u8>, InceptionError> {
        serde_json::to_vec(self).map_err(|_| InceptionError::Code("inception_projection_failed"))
    }
}

pub(crate) fn inspect(context: &crate::context::LiveContext) -> Result<Projection, InceptionError> {
    let input = reader::read(context)?;
    let current_authority = authority_binding(&input.facts);
    Ok(Projection::Current(CurrentProjection {
        schema_version: "ProductInception-v2",
        status: "current",
        context_id: context.context_id().to_owned(),
        candidate: input.candidate,
        current_authority,
        claim_ids: vec![input.facts.claim_id],
        ranking_eligible: false,
        claim_ceiling: "current authority identifies the journey; CL-USABLE-LOOP remains withheld pending same-surface evaluator evidence",
    }))
}

pub(crate) fn current_authority_digest(
    context: &crate::context::LiveContext,
) -> Result<String, InceptionError> {
    reader::current_authority_digest(context)
}

fn authority_binding(facts: &CurrentAuthorityFacts) -> CurrentAuthorityBinding {
    CurrentAuthorityBinding {
        claim_id: facts.claim_id.clone(),
        goal_contract_digest: facts.goal_contract_digest.clone(),
        product_success_contract_digest: facts.product_success_contract_digest.clone(),
        active_plan_digest: facts.active_plan_digest.clone(),
        authority_digest: facts.authority_digest.clone(),
    }
}
