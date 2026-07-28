use super::*;

/// Darwin's status-change timestamp is kernel-maintained and cannot be reset
/// by an unprivileged repository writer. Binding both components to every
/// target and protected object closes same-inode content and metadata ABA that
/// payload or ordinary identity alone cannot distinguish after the original
/// bytes or mode have been restored.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ProtectedChangeVersion {
    pub(crate) ctime_seconds: i64,
    pub(crate) ctime_nanoseconds: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct VersionedProtectedObject {
    pub(crate) object: ObjectRow,
    pub(crate) change_version: ProtectedChangeVersion,
}

pub(crate) struct FenceBudget {
    pub(crate) entries: usize,
    pub(crate) name_bytes: usize,
    pub(crate) bytes: u64,
}

/// Successful mediation output can only be constructed after the accepted
/// kernel effect and both target and context reconciliation have succeeded.
#[derive(Debug, Serialize)]
pub(crate) struct RepositoryFitApplyOutcome {
    pub(crate) schema_version: &'static str,
    pub(crate) outcome_id: String,
    pub(crate) request_id: String,
    pub(crate) plan_sha256: String,
    pub(crate) desired_state_sha256: String,
    pub(crate) verification_sha256: String,
    pub(crate) target_poststate_sha256: String,
    pub(crate) mutation_count: usize,
    pub(crate) status: &'static str,
    pub(crate) effect: &'static str,
    pub(crate) claim_effect: &'static str,
    pub(crate) support_limit: &'static str,
}

impl RepositoryFitApplyOutcome {
    pub(crate) fn mutation_count(&self) -> usize {
        self.mutation_count
    }

    #[cfg(test)]
    pub(crate) fn status(&self) -> &str {
        self.status
    }

    pub(crate) fn outcome_id(&self) -> &str {
        &self.outcome_id
    }
}

pub(crate) enum RepositoryFitApplyFailure<E: RepositoryFitPermitEffects> {
    // Pre-effect refusal retains the request, permit, and lease so callers can
    // recover custody without minting replacement authority. Boxing keeps that
    // recovery-only payload out of the normal terminal error representation.
    PreEffect(Box<PreEffectFailure<E>>),
    Terminal(TerminalApplyFailure),
}

impl<E: RepositoryFitPermitEffects> RepositoryFitApplyFailure<E> {
    #[cfg(test)]
    pub(crate) fn error(&self) -> FitAdapterError {
        match self {
            Self::PreEffect(failure) => failure.error,
            Self::Terminal(failure) => failure.error,
        }
    }

    #[cfg(test)]
    pub(crate) fn effect_started(&self) -> bool {
        matches!(self, Self::Terminal(failure) if failure.effect_started)
    }

    #[cfg(test)]
    pub(crate) fn rollback_complete(&self) -> bool {
        matches!(self, Self::Terminal(failure) if failure.rollback_complete)
    }

    #[cfg(test)]
    pub(crate) fn into_pre_effect(self) -> Option<PreEffectFailure<E>> {
        match self {
            Self::PreEffect(failure) => Some(*failure),
            Self::Terminal(_) => None,
        }
    }

    pub(crate) fn into_settlement(self) -> (FitAdapterError, bool, bool) {
        match self {
            Self::PreEffect(failure) => {
                let PreEffectFailure {
                    error,
                    request,
                    permit,
                    lease,
                } = *failure;
                drop((request, permit, lease));
                (error, false, false)
            }
            Self::Terminal(failure) => (
                failure.error,
                failure.effect_started,
                failure.rollback_complete,
            ),
        }
    }
}

pub(crate) struct PreEffectFailure<E: RepositoryFitPermitEffects> {
    pub(crate) error: FitAdapterError,
    pub(crate) request: OpaqueFitApplyRequest,
    pub(crate) permit: Option<RepositoryFitApplyPermit>,
    pub(crate) lease: Option<RepositoryFitMutationLease<E>>,
}

impl<E: RepositoryFitPermitEffects> PreEffectFailure<E> {
    #[cfg(test)]
    pub(crate) fn into_parts(
        self,
    ) -> (
        OpaqueFitApplyRequest,
        Option<RepositoryFitApplyPermit>,
        Option<RepositoryFitMutationLease<E>>,
    ) {
        (self.request, self.permit, self.lease)
    }
}

pub(crate) struct TerminalApplyFailure {
    pub(crate) error: FitAdapterError,
    pub(crate) effect_started: bool,
    pub(crate) rollback_complete: bool,
}
