use serde::{Deserialize, Serialize};

use super::AdapterErrorId;

pub(super) const INSPECT_SCHEMA: &str = "RepositoryFitInspect-v1";
pub(super) const PLAN_SCHEMA: &str = "RepositoryFitPlan-v1";
pub(super) const VERIFY_SCHEMA: &str = "RepositoryFitVerification-v1";
pub(super) const APPLY_PREPARATION_SCHEMA: &str = "RepositoryFitApplyPreparation-v1";
pub(super) const CLAIM_EFFECT: &str = "none";
pub(super) const SUPPORT_LIMIT: &str =
    "candidate-bound source adapter only; no public activation or live-repository authority";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CandidateProjection {
    pub(super) candidate_id: String,
    pub(super) head_commit: Option<String>,
    pub(super) head_tree: Option<String>,
    pub(super) branch: Option<String>,
    pub(super) status_sha256: String,
    pub(super) worktree_diff_sha256: String,
    pub(super) staged_diff_sha256: String,
    pub(super) untracked_content_sha256: String,
    pub(super) dirty: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TargetProjection {
    pub(super) context_id: String,
    pub(super) repository_root_id: String,
    pub(super) worktree_root_id: String,
    pub(super) candidate: CandidateProjection,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TemplateRowProjection {
    pub(super) source_path: String,
    pub(super) target_path: String,
    pub(super) sha256: String,
    pub(super) byte_length: usize,
    pub(super) unix_mode: u32,
    pub(super) row_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TemplateAuthorityProjection {
    pub(super) manifest_sha256: String,
    pub(super) catalog_sha256: String,
    pub(super) authority_sha256: String,
    pub(super) template_count: usize,
    pub(super) total_bytes: usize,
    pub(super) rows: Vec<TemplateRowProjection>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DesiredProjection {
    pub(super) state_sha256: String,
    pub(super) file_count: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProvenanceProjection {
    pub(super) source: String,
    pub(super) context_id: Option<String>,
    pub(super) candidate_id: Option<String>,
    pub(super) authority_sha256: Option<String>,
    pub(super) row_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ObservedFileProjection {
    pub(super) path: String,
    pub(super) ownership: String,
    pub(super) provenance: ProvenanceProjection,
    pub(super) disposition: String,
    pub(super) observed_sha256: Option<String>,
    pub(super) observed_unix_mode: Option<u32>,
    pub(super) desired_unix_mode: u32,
    pub(super) prior_proof_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InspectionProjection {
    pub(super) mode: String,
    pub(super) classification: String,
    pub(super) compatibility: String,
    pub(super) root_binding: String,
    pub(super) inspection_sha256: String,
    pub(super) files: Vec<ObservedFileProjection>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "sha256")]
pub(crate) enum ExpectedProjection {
    Absent,
    ExactDigest(String),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CheckProjection {
    pub(super) path: String,
    pub(super) expected: ExpectedProjection,
    pub(super) desired_sha256: String,
    pub(super) expected_unix_mode: Option<u32>,
    pub(super) desired_unix_mode: u32,
    pub(super) provenance: ProvenanceProjection,
    pub(super) prior_proof_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RollbackEntryProjection {
    pub(super) disposition: String,
    pub(super) sha256: Option<String>,
    pub(super) byte_length: usize,
    pub(super) unix_mode: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MutationProjection {
    pub(super) path: String,
    pub(super) expected: ExpectedProjection,
    pub(super) replacement_sha256: String,
    pub(super) replacement_byte_length: usize,
    pub(super) replacement_unix_mode: u32,
    pub(super) ownership: String,
    pub(super) provenance: ProvenanceProjection,
    pub(super) prior_proof_sha256: Option<String>,
    pub(super) rollback: RollbackEntryProjection,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConflictProjection {
    pub(super) path: String,
    pub(super) observed_sha256: String,
    pub(super) ownership: String,
    pub(super) provenance: ProvenanceProjection,
    pub(super) prior_proof_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlanProjection {
    pub(super) plan_sha256: String,
    pub(super) authorization_sha256: String,
    pub(super) checks: Vec<CheckProjection>,
    pub(super) mutations: Vec<MutationProjection>,
    pub(super) conflicts: Vec<ConflictProjection>,
    pub(super) rollback_mutation_count: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FitInspectProjection {
    pub(super) schema_version: String,
    pub(super) target: TargetProjection,
    pub(super) authority: TemplateAuthorityProjection,
    pub(super) desired: DesiredProjection,
    pub(super) inspection: InspectionProjection,
    pub(super) effect: String,
    pub(super) claim_effect: String,
    pub(super) support_limit: String,
}

impl FitInspectProjection {
    pub(crate) fn classification(&self) -> &str {
        &self.inspection.classification
    }

    pub(crate) fn compatible(&self) -> bool {
        self.inspection.compatibility == "compatible"
    }

    pub(crate) fn file_count(&self) -> usize {
        self.desired.file_count
    }

    pub(crate) fn to_machine_bytes(&self) -> Result<Vec<u8>, super::FitAdapterError> {
        serde_json::to_vec(self).map_err(|_| super::adapter_error(AdapterErrorId::ProjectionFailed))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FitPlanRecord {
    pub(super) schema_version: String,
    pub(super) target: TargetProjection,
    pub(super) authority: TemplateAuthorityProjection,
    pub(super) desired: DesiredProjection,
    pub(super) inspection: InspectionProjection,
    pub(super) plan: PlanProjection,
    pub(super) effect: String,
    pub(super) claim_effect: String,
    pub(super) support_limit: String,
}

impl FitPlanRecord {
    pub(crate) fn plan_sha256(&self) -> &str {
        &self.plan.plan_sha256
    }

    pub(crate) fn mutation_count(&self) -> usize {
        self.plan.mutations.len()
    }

    pub(crate) fn conflict_count(&self) -> usize {
        self.plan.conflicts.len()
    }

    pub(crate) fn to_machine_bytes(&self) -> Result<Vec<u8>, super::FitAdapterError> {
        serde_json::to_vec(self).map_err(|_| super::adapter_error(AdapterErrorId::ProjectionFailed))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct VerificationFailureProjection {
    pub(super) error_id: String,
    pub(super) classification: String,
    pub(super) compatibility: String,
    pub(super) causal_files: Vec<ObservedFileProjection>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FitVerificationProjection {
    pub(super) schema_version: String,
    pub(super) target: TargetProjection,
    pub(super) authority: TemplateAuthorityProjection,
    pub(super) desired: DesiredProjection,
    pub(super) root_binding: String,
    pub(super) inspection_sha256: String,
    pub(super) matched_files: usize,
    pub(super) idempotent: bool,
    pub(super) byte_verification_sha256: Option<String>,
    pub(super) verification_sha256: Option<String>,
    pub(super) failure: Option<VerificationFailureProjection>,
    pub(super) effect: String,
    pub(super) claim_effect: String,
    pub(super) support_limit: String,
}

impl FitVerificationProjection {
    pub(crate) fn matched_files(&self) -> usize {
        self.matched_files
    }

    pub(crate) fn idempotent(&self) -> bool {
        self.idempotent
    }

    pub(crate) fn to_machine_bytes(&self) -> Result<Vec<u8>, super::FitAdapterError> {
        serde_json::to_vec(self).map_err(|_| super::adapter_error(AdapterErrorId::ProjectionFailed))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FitApplyPreparationProjection {
    pub(super) schema_version: String,
    pub(super) request_id: String,
    pub(super) context_id: String,
    pub(super) candidate_id: String,
    pub(super) root_binding: String,
    pub(super) desired_state_sha256: String,
    pub(super) plan_sha256: String,
    pub(super) accepted_plan_sha256: String,
    pub(super) mutation_count: usize,
    pub(super) effect: String,
    pub(super) claim_effect: String,
    pub(super) support_limit: String,
}

impl FitApplyPreparationProjection {
    pub(crate) fn to_machine_bytes(&self) -> Result<Vec<u8>, super::FitAdapterError> {
        serde_json::to_vec(self).map_err(|_| super::adapter_error(AdapterErrorId::ProjectionFailed))
    }
}
