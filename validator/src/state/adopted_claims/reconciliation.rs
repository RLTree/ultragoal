use super::AdoptedClaimRegistry;
use super::lane_binding::{
    DependencyIdentity, load_declared_dependency_identities, load_exact_dependency_identities,
};
use crate::context::{CandidateIdentity, LiveContext};
use crate::inventory::AuthorityCatalog;
use crate::state::StateError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const AMENDMENT_ID: &str = "AMEND-002";
const AMENDMENT_HASH: &str =
    "sha256:2b4386116e66ded2255fa001341186d4cff1ab883a3e99585be6cc57d5b4cf31";
const GOAL_BYTES: &[u8] = include_bytes!("../../../../GOAL_CONTRACT.md");
const AMENDMENT_BYTES: &[u8] = include_bytes!("../../../../AMENDMENTS.jsonl");
const LANE_BYTES: &[u8] = include_bytes!("../../../../LANE_REGISTRY.json");

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
    amendment_id: &'static str,
    amendment_hash: &'static str,
    lane_registry_sha256: String,
    source_binding: SourceBindingStatus,
    dependency_identities: Vec<DependencyIdentity>,
    decisions: Vec<StagedClaimDecision>,
    generated_outputs: Vec<String>,
    fixtures: Vec<String>,
    effects: Vec<String>,
}

#[derive(Deserialize)]
struct AmendmentRow {
    amendment_id: String,
    amendment_hash: String,
    new_contract_hash: String,
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
        claim_registry_sha256,
        contract_manifest_sha256,
        handoff_sha256,
        dependency_identities,
        SourceBindingStatus::RegistryDeclared,
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
    let dependency_identities = load_exact_dependency_identities(LANE_BYTES, context.candidate())?;
    let staged = stage(
        context,
        authority_catalog,
        registry,
        claim_registry_sha256,
        contract_manifest_sha256,
        handoff_sha256,
        dependency_identities,
        SourceBindingStatus::RootSourceVerified,
    )?;
    Ok(RootClaimStage {
        stage_id: staged.stage_id()?.to_owned(),
    })
}

fn stage(
    context: &LiveContext,
    authority_catalog: &AuthorityCatalog,
    registry: &AdoptedClaimRegistry,
    claim_registry_sha256: String,
    contract_manifest_sha256: String,
    handoff_sha256: String,
    dependency_identities: Vec<DependencyIdentity>,
    source_binding: SourceBindingStatus,
) -> Result<StagedClaimReconciliation, StateError> {
    let goal_contract_sha256 = sha256(GOAL_BYTES);
    verify_amendment(&goal_contract_sha256)?;
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
        claim_registry_sha256,
        contract_manifest_sha256,
        handoff_sha256,
        goal_contract_sha256,
        amendments_sha256: sha256(AMENDMENT_BYTES),
        amendment_id: AMENDMENT_ID,
        amendment_hash: AMENDMENT_HASH,
        lane_registry_sha256: sha256(LANE_BYTES),
        source_binding,
        dependency_identities,
        decisions,
        generated_outputs: Vec::new(),
        fixtures: Vec::new(),
        effects: Vec::new(),
    };
    staged.stage_id = Some(digest(&staged)?);
    Ok(staged)
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

fn verify_amendment(goal_contract_sha256: &str) -> Result<(), StateError> {
    let rows = std::str::from_utf8(AMENDMENT_BYTES)
        .map_err(|_| invalid("adopted-amendment-log-invalid"))?
        .lines()
        .map(serde_json::from_str::<AmendmentRow>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| invalid("adopted-amendment-log-invalid"))?;
    let row = rows
        .iter()
        .find(|row| row.amendment_id == AMENDMENT_ID)
        .ok_or_else(|| invalid("adopted-amendment-missing"))?;
    if row.amendment_hash != AMENDMENT_HASH || row.new_contract_hash != goal_contract_sha256 {
        return Err(invalid("adopted-amendment-identity-mismatch"));
    }
    Ok(())
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
