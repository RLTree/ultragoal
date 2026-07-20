use super::*;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FitPlanRecord {
    pub(crate) schema_version: String,
    pub(crate) target: TargetProjection,
    pub(crate) authority: TemplateAuthorityProjection,
    pub(crate) desired: DesiredProjection,
    pub(crate) inspection: InspectionProjection,
    pub(crate) local_state: LocalStatePolicyProjection,
    pub(crate) plan: PlanProjection,
    pub(crate) effect: String,
    pub(crate) claim_effect: String,
    pub(crate) support_limit: String,
}

impl FitPlanRecord {
    pub(crate) fn plan_sha256(&self) -> &str {
        &self.plan.plan_sha256
    }

    pub(crate) fn mutation_count(&self) -> usize {
        self.plan.mutations.len() + usize::from(self.plan.local_state.mutation_required)
    }

    pub(crate) fn conflict_count(&self) -> usize {
        self.plan.conflicts.len()
    }

    pub(crate) fn to_machine_bytes(&self) -> Result<Vec<u8>, super::super::FitAdapterError> {
        serde_json::to_vec(self)
            .map_err(|_| super::super::adapter_error(AdapterErrorId::ProjectionFailed))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct VerificationFailureProjection {
    pub(crate) error_id: String,
    pub(crate) classification: String,
    pub(crate) compatibility: String,
    pub(crate) causal_files: Vec<ObservedFileProjection>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FitVerificationProjection {
    pub(crate) schema_version: String,
    pub(crate) target: TargetProjection,
    pub(crate) authority: TemplateAuthorityProjection,
    pub(crate) desired: DesiredProjection,
    pub(crate) local_state: LocalStatePolicyProjection,
    pub(crate) root_binding: String,
    pub(crate) inspection_sha256: String,
    pub(crate) matched_files: usize,
    pub(crate) idempotent: bool,
    pub(crate) byte_verification_sha256: Option<String>,
    pub(crate) verification_sha256: Option<String>,
    pub(crate) failure: Option<VerificationFailureProjection>,
    pub(crate) effect: String,
    pub(crate) claim_effect: String,
    pub(crate) support_limit: String,
}

impl FitVerificationProjection {
    pub(crate) fn matched_files(&self) -> usize {
        self.matched_files
    }

    pub(crate) fn idempotent(&self) -> bool {
        self.idempotent
    }

    pub(crate) fn to_machine_bytes(&self) -> Result<Vec<u8>, super::super::FitAdapterError> {
        serde_json::to_vec(self)
            .map_err(|_| super::super::adapter_error(AdapterErrorId::ProjectionFailed))
    }
}

#[cfg(test)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FitApplyPreparationProjection {
    pub(crate) schema_version: String,
    pub(crate) request_id: String,
    pub(crate) context_id: String,
    pub(crate) candidate_id: String,
    pub(crate) root_binding: String,
    pub(crate) desired_state_sha256: String,
    pub(crate) plan_sha256: String,
    pub(crate) accepted_plan_sha256: String,
    pub(crate) mutation_count: usize,
    pub(crate) effect: String,
    pub(crate) claim_effect: String,
    pub(crate) support_limit: String,
}
