use super::*;

pub(crate) const MAX_PLAN_RECORD_BYTES: usize = 16 * 1024 * 1024;
pub(crate) const REQUEST_STAGE_PREPARED: u8 = 0;
pub(crate) const REQUEST_STAGE_IN_FLIGHT: u8 = 1;
pub(crate) const REQUEST_STAGE_SETTLED: u8 = 2;
pub(crate) const REQUEST_STAGE_ROLLED_BACK: u8 = 3;
pub(crate) const REQUEST_STAGE_AMBIGUOUS: u8 = 4;

pub(crate) static NEXT_APPLY_REQUEST_ISSUANCE: AtomicU64 = AtomicU64::new(1);

pub(crate) struct CurrentPlan {
    pub(crate) target: TargetProjection,
    pub(crate) bundle: DesiredBundle,
    pub(crate) inspection: FitInspection,
    pub(crate) observed_modes: BTreeMap<String, Option<u32>>,
    pub(crate) plan: FitPlan,
}

/// Process-local capability state shared only by one prepared request, its
/// root-issued permit, and its exclusive mutation lease. The stage transition
/// is the one linearization point for apply authority.
pub(crate) struct ApplyRequestSeal {
    pub(crate) issuance: u64,
    pub(crate) seal_id: String,
    pub(crate) stage: AtomicU8,
}

impl ApplyRequestSeal {
    pub(crate) fn new(issuance: u64, seal_id: String) -> Self {
        Self {
            issuance,
            seal_id,
            stage: AtomicU8::new(REQUEST_STAGE_PREPARED),
        }
    }

    pub(crate) const fn issuance(&self) -> u64 {
        self.issuance
    }

    pub(crate) fn matches(&self, expected: &str) -> bool {
        self.seal_id == expected
    }

    pub(crate) fn begin(&self) -> Result<(), FitAdapterError> {
        self.stage
            .compare_exchange(
                REQUEST_STAGE_PREPARED,
                REQUEST_STAGE_IN_FLIGHT,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map(|_| ())
            .map_err(|_| adapter_error(AdapterErrorId::ApplyPermitReplayed))
    }

    pub(crate) fn settle(&self) -> Result<(), FitAdapterError> {
        self.transition(REQUEST_STAGE_SETTLED)
    }

    pub(crate) fn rolled_back(&self) -> Result<(), FitAdapterError> {
        self.transition(REQUEST_STAGE_ROLLED_BACK)
    }

    pub(crate) fn ambiguous(&self) -> Result<(), FitAdapterError> {
        self.transition(REQUEST_STAGE_AMBIGUOUS)
    }

    pub(crate) fn transition(&self, next: u8) -> Result<(), FitAdapterError> {
        self.stage
            .compare_exchange(
                REQUEST_STAGE_IN_FLIGHT,
                next,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map(|_| ())
            .map_err(|_| adapter_error(AdapterErrorId::ApplyOutcomeInvalid))
    }

    #[cfg(test)]
    pub(crate) fn stage_for_test(&self) -> u8 {
        self.stage.load(Ordering::Acquire)
    }
}

/// A one-use, non-cloneable accepted-plan carrier. No method on this type
/// performs an effect; a separate root-owned permit boundary must consume it.
pub(crate) struct OpaqueFitApplyRequest {
    pub(crate) request_id: String,
    pub(crate) context_id: String,
    pub(crate) candidate_id: String,
    pub(crate) root_binding: String,
    pub(crate) accepted_plan_sha256: String,
    pub(crate) plan_record_bytes: Vec<u8>,
    pub(crate) target: TargetProjection,
    pub(crate) authority: super::super::projection::TemplateAuthorityProjection,
    pub(crate) desired: DesiredState,
    pub(crate) observed_modes: BTreeMap<String, Option<u32>>,
    pub(crate) plan: FitPlan,
    pub(crate) authorization: PlanAuthorization,
    pub(crate) unix_modes: BTreeMap<String, u32>,
    pub(crate) seal: Arc<ApplyRequestSeal>,
}

impl OpaqueFitApplyRequest {
    pub(crate) fn request_id(&self) -> &str {
        &self.request_id
    }

    pub(crate) fn context_id(&self) -> &str {
        &self.context_id
    }

    pub(crate) fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    pub(crate) fn root_binding(&self) -> &str {
        &self.root_binding
    }

    pub(crate) fn plan_sha256(&self) -> &str {
        self.plan.plan_sha256()
    }

    pub(crate) fn unix_modes(&self) -> &BTreeMap<String, u32> {
        &self.unix_modes
    }

    pub(crate) fn seal_matches(&self, expected: &str) -> bool {
        self.seal.matches(expected)
    }

    pub(crate) fn seal_id(&self) -> String {
        request_seal_id(
            &self.request_id,
            &self.context_id,
            &self.candidate_id,
            self.seal.issuance(),
        )
    }

    #[cfg(test)]
    pub(crate) fn duplicate_for_test(&self) -> Self {
        Self {
            request_id: self.request_id.clone(),
            context_id: self.context_id.clone(),
            candidate_id: self.candidate_id.clone(),
            root_binding: self.root_binding.clone(),
            accepted_plan_sha256: self.accepted_plan_sha256.clone(),
            plan_record_bytes: self.plan_record_bytes.clone(),
            target: self.target.clone(),
            authority: self.authority.clone(),
            desired: self.desired.clone(),
            observed_modes: self.observed_modes.clone(),
            plan: self.plan.clone(),
            authorization: self.authorization.clone(),
            unix_modes: self.unix_modes.clone(),
            seal: Arc::clone(&self.seal),
        }
    }

    #[cfg(test)]
    pub(crate) fn execute_for_test(
        self,
        effects: &mut impl crate::repository_fit::FitEffects,
    ) -> Result<crate::repository_fit::FitVerification, crate::repository_fit::FitError> {
        crate::repository_fit::apply(&self.plan, &self.authorization, effects)?;
        crate::repository_fit::verify(&self.desired, effects)
    }
}

pub(crate) struct PreparedFitApply {
    pub(crate) request: OpaqueFitApplyRequest,
    pub(crate) projection: FitApplyPreparationProjection,
}

impl PreparedFitApply {
    pub(crate) fn request(&self) -> &OpaqueFitApplyRequest {
        &self.request
    }

    pub(crate) fn projection(&self) -> &FitApplyPreparationProjection {
        &self.projection
    }

    pub(crate) fn into_request(self) -> OpaqueFitApplyRequest {
        self.request
    }
}
