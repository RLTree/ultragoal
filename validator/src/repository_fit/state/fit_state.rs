use super::*;

impl FitPlan {
    pub fn plan_sha256(&self) -> &str {
        &self.plan_sha256
    }

    pub fn mutations(&self) -> &[Mutation] {
        &self.mutations
    }

    pub fn checks(&self) -> &[FitCheck] {
        &self.checks
    }

    pub fn conflicts(&self) -> &[FitConflict] {
        &self.conflicts
    }

    pub fn rollback_plan(&self) -> &RollbackPlan {
        &self.rollback
    }

    pub(crate) fn all_mutations(&self) -> Vec<Mutation> {
        self.mutations
            .iter()
            .cloned()
            .chain(
                self.local_state
                    .as_ref()
                    .and_then(|state| state.mutation.as_ref())
                    .cloned(),
            )
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PlanAuthorization {
    pub(crate) context_id: String,
    pub(crate) candidate_id: String,
    pub(crate) plan_sha256: String,
}

impl PlanAuthorization {
    pub fn new(
        context_id: String,
        candidate_id: String,
        plan_sha256: String,
    ) -> Result<Self, FitError> {
        if [&context_id, &candidate_id, &plan_sha256]
            .into_iter()
            .any(|value| !valid_digest(value))
        {
            return Err(error(FitErrorId::InvalidSpec));
        }
        Ok(Self {
            context_id,
            candidate_id,
            plan_sha256,
        })
    }

    pub(crate) fn matches(&self, plan: &FitPlan) -> bool {
        self.context_id == plan.context_id
            && self.candidate_id == plan.candidate_id
            && self.plan_sha256 == plan.plan_sha256
    }
}

#[derive(Clone, Debug)]
pub struct AppliedFit {
    pub(crate) plan_sha256: String,
    pub(crate) root_binding: String,
    pub(crate) mutations: Vec<Mutation>,
}

impl AppliedFit {
    pub fn plan_sha256(&self) -> &str {
        &self.plan_sha256
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FitVerification {
    pub(crate) context_id: String,
    pub(crate) candidate_id: String,
    pub(crate) desired_state_sha256: String,
    pub(crate) root_binding: String,
    pub(crate) matched_files: usize,
    pub(crate) idempotent: bool,
    pub(crate) verification_sha256: String,
}

impl FitVerification {
    pub const fn matched_files(&self) -> usize {
        self.matched_files
    }

    pub const fn idempotent(&self) -> bool {
        self.idempotent
    }
}
