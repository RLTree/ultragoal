mod model;
mod normalize;
mod parser;
mod ranking;
mod reader;
mod validation;

#[cfg(test)]
mod tests;

use model::{BriefV1, BriefV2, CandidateBinding, ContractBinding, ContractFacts};
use parser::ParsedBrief;
use serde::Serialize;

pub(crate) use ranking::{RankingDisposition, bind_actions};

const MISSING_FIELDS: &[&str] = &[
    "schema",
    "product_success_contract_id",
    "product_success_contract_digest",
    "claim_ids",
    "target_problem",
    "audience",
    "job_to_be_done",
    "context_of_use",
    "desired_outcome",
    "first_value_event",
    "operator.kind",
    "operator.actor_reference",
    "real_work.repository_identity",
    "real_work.starting_candidate",
    "real_work.dirty_state_expectation",
    "real_work.task_id",
    "real_work.task",
    "real_work.expected_useful_outcome",
    "public_entry_surface.surface_id",
    "public_entry_surface.route",
    "public_entry_surface.forbidden_bypasses",
    "protected_invariants",
    "first_truth_loop",
    "depth_triggers",
    "evidence_class",
    "evidence_ladder",
    "claim_ceiling",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum InceptionError {
    Context,
    CatalogUnavailable,
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
            Self::Context => "the live context changed during inception inspection",
            Self::CatalogUnavailable => "the canonical authority catalog is unavailable",
            Self::ContractBindingInvalid => "the inception contract bindings are stale or invalid",
            Self::UnsafeInput => "the Product Success Brief input is not a safe regular file",
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
    pub(crate) context_id: String,
    pub(crate) candidate: CandidateBinding,
    pub(crate) contract: ContractBinding,
    pub(crate) authority_catalog_id: String,
    pub(crate) brief_path: &'static str,
    pub(crate) missing_fields: Vec<&'static str>,
    pub(crate) claim_ceiling: &'static str,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HistoricalProjection {
    pub(crate) schema_version: &'static str,
    pub(crate) status: &'static str,
    pub(crate) context_id: String,
    pub(crate) candidate: CandidateBinding,
    pub(crate) contract: ContractBinding,
    pub(crate) authority_catalog_id: String,
    pub(crate) brief_digest: String,
    pub(crate) brief: BriefV1,
    pub(crate) ranking_eligible: bool,
    pub(crate) claim_ceiling: &'static str,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActiveProjection {
    pub(crate) schema_version: &'static str,
    pub(crate) status: &'static str,
    pub(crate) context_id: String,
    pub(crate) candidate: CandidateBinding,
    pub(crate) contract: ContractBinding,
    pub(crate) authority_catalog_id: String,
    pub(crate) brief_digest: String,
    pub(crate) brief: BriefV2,
    pub(crate) ranking_eligible: bool,
    pub(crate) claim_ceiling: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub(crate) enum Projection {
    Missing(MissingProjection),
    Historical(HistoricalProjection),
    Active(ActiveProjection),
}

impl Projection {
    pub(crate) fn to_json(&self) -> Result<Vec<u8>, InceptionError> {
        serde_json::to_vec(self).map_err(|_| InceptionError::Code("inception_projection_failed"))
    }
}

pub(crate) fn inspect(context: &crate::context::LiveContext) -> Result<Projection, InceptionError> {
    let input = reader::read(context)?;
    let contract = contract_binding(&input.facts);
    let Some(bytes) = input.brief else {
        return Ok(Projection::Missing(MissingProjection {
            schema_version: "ProductInceptionInput-v1",
            status: "missing",
            context_id: context.context_id().to_owned(),
            candidate: input.candidate,
            contract,
            authority_catalog_id: input.catalog.catalog_id().to_owned(),
            brief_path: reader::BRIEF_PATH,
            missing_fields: MISSING_FIELDS.to_vec(),
            claim_ceiling: "inception input missing; evidence-led ranking remains withheld",
        }));
    };
    let brief_digest = crate::digest::bytes(&bytes);
    match parser::parse(&bytes)? {
        ParsedBrief::Historical(mut brief) => {
            normalize::v1(&mut brief);
            validation::validate_v1(&brief, &input.facts).map_err(InceptionError::Code)?;
            Ok(Projection::Historical(HistoricalProjection {
                schema_version: "ProductInceptionHistory-v1",
                status: "historical",
                context_id: context.context_id().to_owned(),
                candidate: input.candidate,
                contract,
                authority_catalog_id: input.catalog.catalog_id().to_owned(),
                brief_digest,
                brief: *brief,
                ranking_eligible: false,
                claim_ceiling: "historical v1 is readable history; evidence-led ranking remains withheld",
            }))
        }
        ParsedBrief::EvidenceLed(mut brief) => {
            normalize::v2(&mut brief);
            validation::validate_v2(&brief, &input.facts, &input.candidate)
                .map_err(InceptionError::Code)?;
            Ok(Projection::Active(ActiveProjection {
                schema_version: "ProductInception-v1",
                status: "active",
                context_id: context.context_id().to_owned(),
                candidate: input.candidate,
                contract,
                authority_catalog_id: input.catalog.catalog_id().to_owned(),
                brief_digest,
                brief: *brief,
                ranking_eligible: true,
                claim_ceiling: "source-local inception projection only; no claim promotion"
                    .to_owned(),
            }))
        }
    }
}

fn contract_binding(facts: &ContractFacts) -> ContractBinding {
    ContractBinding {
        product_success_contract_id: facts.product_contract_id.clone(),
        contract_version: facts.contract_version.clone(),
        product_success_contract_digest: facts.contract_digest.clone(),
        authority_contract_id: facts.authority_contract_id.clone(),
        claim_registry_digest: facts.claim_registry_digest.clone(),
        public_surface_catalog_digest: facts.public_surface_catalog_digest.clone(),
    }
}
