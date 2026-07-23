use super::AdoptedClaimRegistry;
use super::lane_binding::{
    DependencyIdentity, load_declared_dependency_identities, load_root_dependency_identities,
};
use crate::context::{CandidateIdentity, LiveContext};
use crate::contract_amendment::{
    CurrentAmendmentBinding, ExpectedArtifactBinding, ValidatedCurrentAmendment, validate_current,
};
use crate::inventory::AuthorityCatalog;
use crate::state::StateError;
use serde::Serialize;
use sha2::{Digest, Sha256};

const AMENDMENT_ID: &str = "AMEND-005";
const AMENDMENT_HASH: &str =
    "sha256:39f51d83b90508087e45459f81add8094c2c455fb669e9bbd5885b3909d5338d";
const PREVIOUS_CONTRACT_HASH: &str =
    "sha256:20dadce2e50ef92fa4f19614f8fc70ae472ad32064561fca2208ca213cf0685b";
const GOAL_BYTES: &[u8] = include_bytes!("../../../../GOAL_CONTRACT.md");
const AMENDMENT_BYTES: &[u8] = include_bytes!("../../../../AMENDMENTS.jsonl");
const PRODUCT_CONTRACT_BYTES: &[u8] =
    include_bytes!("../../../../examples/generated/PRODUCT_SUCCESS_CONTRACT.json");
const PRODUCT_BRIEF_BYTES: &[u8] = include_bytes!("../../../../PRODUCT_SUCCESS_BRIEF.json");
const ADVISORY_DECISION_BYTES: &[u8] = include_bytes!(
    "../../../../docs/ultragoal-successor-live/root-decisions/AGENTIC-ENGINEERING-V3-LIFECYCLE-ADVISORY-004.json"
);
const LANE_BYTES: &[u8] = include_bytes!("../../../../LANE_REGISTRY.json");
const PRODUCT_CONTRACT_PATH: &str = "examples/generated/PRODUCT_SUCCESS_CONTRACT.json";
const PRODUCT_BRIEF_PATH: &str = "PRODUCT_SUCCESS_BRIEF.json";
const ADVISORY_DECISION_PATH: &str = "docs/ultragoal-successor-live/root-decisions/AGENTIC-ENGINEERING-V3-LIFECYCLE-ADVISORY-004.json";

#[derive(Serialize)]
pub(in crate::state) struct StagedClaimReconciliation {
    schema_version: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    stage_id: Option<String>,
    context_id: String,
    candidate: CandidateIdentity,
    authority_catalog_id: String,
    claim_registry_sha256: String,
    contract_manifest_sha256: String,
    handoff_sha256: String,
    goal_contract_sha256: String,
    amendments_sha256: String,
    amendment_id: String,
    amendment_hash: String,
    lane_registry_sha256: String,
    source_binding: SourceBindingStatus,
    dependency_identities: Vec<DependencyIdentity>,
    decisions: Vec<StagedClaimDecision>,
    generated_outputs: Vec<String>,
    fixtures: Vec<String>,
    effects: Vec<String>,
}

#[derive(Serialize)]
struct StagedClaimDecision {
    claim_id: String,
    disposition: ClaimDisposition,
    reason: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum ClaimDisposition {
    Withheld,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum SourceBindingStatus {
    RegistryDeclared,
    RootSourceVerified,
}

pub(crate) struct RootClaimStage {
    stage_id: String,
}

pub(in crate::state) fn stage_target(
    context: &LiveContext,
    authority_catalog: &AuthorityCatalog,
    registry: &AdoptedClaimRegistry,
    claim_registry_sha256: String,
    contract_manifest_sha256: String,
    handoff_sha256: String,
) -> Result<StagedClaimReconciliation, StateError> {
    let dependency_identities = load_declared_dependency_identities(LANE_BYTES)?;
    stage(
        context,
        authority_catalog,
        registry,
        StageInputs {
            claim_registry_sha256,
            contract_manifest_sha256,
            handoff_sha256,
            dependency_identities,
            source_binding: SourceBindingStatus::RegistryDeclared,
        },
    )
}

pub(in crate::state) fn stage_root(
    context: &LiveContext,
    authority_catalog: &AuthorityCatalog,
    registry: &AdoptedClaimRegistry,
    claim_registry_sha256: String,
    contract_manifest_sha256: String,
    handoff_sha256: String,
) -> Result<RootClaimStage, StateError> {
    let dependency_identities = load_root_dependency_identities(LANE_BYTES, context)?;
    let staged = stage(
        context,
        authority_catalog,
        registry,
        StageInputs {
            claim_registry_sha256,
            contract_manifest_sha256,
            handoff_sha256,
            dependency_identities,
            source_binding: SourceBindingStatus::RootSourceVerified,
        },
    )?;
    Ok(RootClaimStage {
        stage_id: staged.stage_id()?.to_owned(),
    })
}

fn stage(
    context: &LiveContext,
    authority_catalog: &AuthorityCatalog,
    registry: &AdoptedClaimRegistry,
    inputs: StageInputs,
) -> Result<StagedClaimReconciliation, StateError> {
    let goal_contract_sha256 = sha256(GOAL_BYTES);
    let amendment = verify_amendment(&goal_contract_sha256)?;
    let decisions = registry
        .claims
        .iter()
        .map(|claim| StagedClaimDecision {
            claim_id: claim.claim_id.clone(),
            disposition: ClaimDisposition::Withheld,
            reason: if claim.claim_id == "CL-EVAL-IMPROVEMENT" {
                "n11-positive-execution-externally-blocked"
            } else {
                "n12-a-staged-no-claim-promotion"
            },
        })
        .collect();
    let mut staged = StagedClaimReconciliation {
        schema_version: "HarnessStagedClaimReconciliation-v1",
        stage_id: None,
        context_id: context.context_id().to_owned(),
        candidate: context.candidate().clone(),
        authority_catalog_id: authority_catalog.catalog_id().to_owned(),
        claim_registry_sha256: inputs.claim_registry_sha256,
        contract_manifest_sha256: inputs.contract_manifest_sha256,
        handoff_sha256: inputs.handoff_sha256,
        goal_contract_sha256,
        amendments_sha256: sha256(AMENDMENT_BYTES),
        amendment_id: amendment.amendment_id().to_owned(),
        amendment_hash: amendment.amendment_hash().to_owned(),
        lane_registry_sha256: sha256(LANE_BYTES),
        source_binding: inputs.source_binding,
        dependency_identities: inputs.dependency_identities,
        decisions,
        generated_outputs: Vec::new(),
        fixtures: Vec::new(),
        effects: Vec::new(),
    };
    staged.stage_id = Some(digest(&staged)?);
    Ok(staged)
}

struct StageInputs {
    claim_registry_sha256: String,
    contract_manifest_sha256: String,
    handoff_sha256: String,
    dependency_identities: Vec<DependencyIdentity>,
    source_binding: SourceBindingStatus,
}

impl RootClaimStage {
    pub(crate) fn stage_id(&self) -> &str {
        &self.stage_id
    }
}

impl StagedClaimReconciliation {
    pub(in crate::state) fn stage_id(&self) -> Result<&str, StateError> {
        self.stage_id
            .as_deref()
            .ok_or_else(|| invalid("staged-claim-identity-missing"))
    }
}

fn verify_amendment(goal_contract_sha256: &str) -> Result<ValidatedCurrentAmendment, StateError> {
    let output_hash = sha256(PRODUCT_CONTRACT_BYTES);
    let brief_hash = sha256(PRODUCT_BRIEF_BYTES);
    let advisory_decision_hash = sha256(ADVISORY_DECISION_BYTES);
    validate_current(
        AMENDMENT_BYTES,
        CurrentAmendmentBinding {
            amendment_id: AMENDMENT_ID,
            amendment_hash: AMENDMENT_HASH,
            previous_contract_hash: PREVIOUS_CONTRACT_HASH,
            new_contract_hash: goal_contract_sha256,
            change_class: "strengthens",
            backlog_updates: &[
                ExpectedArtifactBinding {
                    path: PRODUCT_CONTRACT_PATH,
                    digest: &output_hash,
                },
                ExpectedArtifactBinding {
                    path: PRODUCT_BRIEF_PATH,
                    digest: &brief_hash,
                },
                ExpectedArtifactBinding {
                    path: ADVISORY_DECISION_PATH,
                    digest: &advisory_decision_hash,
                },
            ],
        },
    )
    .map_err(|code| invalid(&format!("adopted-amendment-invalid:{code}")))
}

fn digest(value: &impl Serialize) -> Result<String, StateError> {
    let bytes =
        serde_json::to_vec(value).map_err(|_| invalid("staged-claim-serialization-failed"))?;
    Ok(sha256(&bytes))
}

fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn invalid(code: &str) -> StateError {
    StateError::InvalidCatalog(code.to_owned())
}
