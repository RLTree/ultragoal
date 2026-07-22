use super::*;

pub(crate) const INSPECT_SCHEMA: &str = "RepositoryFitInspect-v1";
pub(crate) const PLAN_SCHEMA: &str = "RepositoryFitPlan-v1";
pub(crate) const VERIFY_SCHEMA: &str = "RepositoryFitVerification-v1";
#[cfg(test)]
pub(crate) const APPLY_PREPARATION_SCHEMA: &str = "RepositoryFitApplyPreparation-v1";
pub(crate) const CLAIM_EFFECT: &str = "none";
pub(crate) const SUPPORT_LIMIT: &str = "source-built public repository-fit adapter; effectful apply requires a supported Darwin host and preprovisioned owner-only local authority state; install and live-user proof remain separate";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CandidateProjection {
    pub(crate) candidate_id: String,
    pub(crate) head_commit: Option<String>,
    pub(crate) head_tree: Option<String>,
    pub(crate) branch: Option<String>,
    pub(crate) status_sha256: String,
    pub(crate) worktree_diff_sha256: String,
    pub(crate) staged_diff_sha256: String,
    pub(crate) untracked_content_sha256: String,
    pub(crate) dirty: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TargetProjection {
    pub(crate) context_id: String,
    pub(crate) repository_root_id: String,
    pub(crate) worktree_root_id: String,
    pub(crate) candidate: CandidateProjection,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TemplateRowProjection {
    pub(crate) source_path: String,
    pub(crate) target_path: String,
    pub(crate) sha256: String,
    pub(crate) byte_length: usize,
    pub(crate) unix_mode: u32,
    pub(crate) row_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TemplateAuthorityProjection {
    pub(crate) manifest_sha256: String,
    pub(crate) catalog_sha256: String,
    pub(crate) authority_sha256: String,
    pub(crate) template_count: usize,
    pub(crate) total_bytes: usize,
    pub(crate) rows: Vec<TemplateRowProjection>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DesiredProjection {
    pub(crate) state_sha256: String,
    pub(crate) file_count: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProvenanceProjection {
    pub(crate) source: String,
    pub(crate) context_id: Option<String>,
    pub(crate) candidate_id: Option<String>,
    pub(crate) authority_sha256: Option<String>,
    pub(crate) row_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ObservedFileProjection {
    pub(crate) path: String,
    pub(crate) ownership: String,
    pub(crate) provenance: ProvenanceProjection,
    pub(crate) disposition: String,
    pub(crate) observed_sha256: Option<String>,
    pub(crate) observed_unix_mode: Option<u32>,
    pub(crate) desired_unix_mode: u32,
    pub(crate) prior_proof_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InspectionProjection {
    pub(crate) mode: String,
    pub(crate) classification: String,
    pub(crate) compatibility: String,
    pub(crate) root_binding: String,
    pub(crate) inspection_sha256: String,
    pub(crate) files: Vec<ObservedFileProjection>,
    pub(crate) local_state: Option<LocalStatePolicyProjection>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LocalStatePolicyProjection {
    pub(crate) path: String,
    pub(crate) required_rule: String,
    pub(crate) disposition: String,
    pub(crate) observed_sha256: Option<String>,
    pub(crate) observed_unix_mode: Option<u32>,
    pub(crate) desired_unix_mode: u32,
    pub(crate) desired_sha256: String,
    pub(crate) replacement_sha256: String,
    pub(crate) replacement_byte_length: usize,
    pub(crate) mutation_required: bool,
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
    pub(crate) path: String,
    pub(crate) expected: ExpectedProjection,
    pub(crate) desired_sha256: String,
    pub(crate) expected_unix_mode: Option<u32>,
    pub(crate) desired_unix_mode: u32,
    pub(crate) provenance: ProvenanceProjection,
    pub(crate) prior_proof_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RollbackEntryProjection {
    pub(crate) disposition: String,
    pub(crate) sha256: Option<String>,
    pub(crate) byte_length: usize,
    pub(crate) unix_mode: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MutationProjection {
    pub(crate) path: String,
    pub(crate) expected: ExpectedProjection,
    pub(crate) replacement_sha256: String,
    pub(crate) replacement_byte_length: usize,
    pub(crate) replacement_unix_mode: u32,
    pub(crate) ownership: String,
    pub(crate) provenance: ProvenanceProjection,
    pub(crate) prior_proof_sha256: Option<String>,
    pub(crate) rollback: RollbackEntryProjection,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConflictProjection {
    pub(crate) path: String,
    pub(crate) observed_sha256: String,
    pub(crate) ownership: String,
    pub(crate) provenance: ProvenanceProjection,
    pub(crate) prior_proof_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlanProjection {
    pub(crate) plan_sha256: String,
    pub(crate) authorization_sha256: String,
    pub(crate) checks: Vec<CheckProjection>,
    pub(crate) mutations: Vec<MutationProjection>,
    pub(crate) conflicts: Vec<ConflictProjection>,
    pub(crate) local_state: Option<LocalStatePolicyProjection>,
    pub(crate) rollback_mutation_count: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FitInspectProjection {
    pub(crate) schema_version: String,
    pub(crate) target: TargetProjection,
    pub(crate) authority: TemplateAuthorityProjection,
    pub(crate) desired: DesiredProjection,
    pub(crate) inspection: InspectionProjection,
    pub(crate) local_state: LocalStatePolicyProjection,
    pub(crate) effect: String,
    pub(crate) claim_effect: String,
    pub(crate) support_limit: String,
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

    pub(crate) fn to_machine_bytes(&self) -> Result<Vec<u8>, super::super::FitAdapterError> {
        serde_json::to_vec(self)
            .map_err(|_| super::super::adapter_error(AdapterErrorId::ProjectionFailed))
    }
}
