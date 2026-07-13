//! Candidate-bound repository-fit product adapter.
//!
//! This module compiles desired state from bytes embedded in the validator,
//! produces direct typed machine projections, and prepares an opaque accepted
//! apply request. It owns no public route, root effect grant, apply permit, or
//! live-repository authority.

mod authority;
mod catalog;
mod ledger;
mod model;
mod protocol;
mod root_permit;

use serde::{Deserialize, Serialize};

use super::{FitError, FitErrorId};

pub(in crate::repository_fit) use authority::LocalMutationGrant;
pub(crate) use authority::{
    execute_prepared_apply, RepositoryFitApplyNonce, RepositoryFitAuthorityStore,
    RepositoryFitProductionOutcome, RepositoryFitTrustedClock,
};
pub(crate) use model::{
    FitApplyPreparationProjection, FitInspectProjection, FitPlanRecord, FitVerificationProjection,
};
pub(crate) use protocol::{
    inspect_target, plan_target, prepare_apply_request, verify_target, OpaqueFitApplyRequest,
    PreparedFitApply,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AdapterErrorId {
    ContextStale,
    InvalidTemplateCatalog,
    TargetUnavailable,
    ProjectionFailed,
    InvalidPlanRecord,
    StalePlan,
    AcceptanceMismatch,
    PlanConflict,
    UnsupportedHost,
    EffectFailed,
    ApplyPermitMissing,
    ApplyPermitInvalid,
    ApplyPermitExpired,
    ApplyPermitReplayed,
    ApplyLeaseInvalid,
    ApplyMutationScopeViolation,
    ApplyOutcomeInvalid,
    ApplyRolledBack,
    ApplyOutcomeAmbiguous,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct FitAdapterError {
    id: AdapterErrorId,
    kernel_error_id: Option<FitErrorId>,
}

impl FitAdapterError {
    pub(crate) const fn id(self) -> AdapterErrorId {
        self.id
    }

    pub(crate) const fn kernel_error_id(self) -> Option<FitErrorId> {
        self.kernel_error_id
    }

    pub(crate) const fn cause(self) -> &'static str {
        match self.id {
            AdapterErrorId::ContextStale => "the target LiveContext or candidate is stale",
            AdapterErrorId::InvalidTemplateCatalog => {
                "the compiled template catalog does not match its manifest authority"
            }
            AdapterErrorId::TargetUnavailable => {
                "the target repository could not be opened through the anchored reader"
            }
            AdapterErrorId::ProjectionFailed => {
                "the bounded repository-fit machine projection could not be encoded"
            }
            AdapterErrorId::InvalidPlanRecord => {
                "the supplied plan is not one exact canonical bounded plan record"
            }
            AdapterErrorId::StalePlan => {
                "the supplied plan does not equal the recomputed current target plan"
            }
            AdapterErrorId::AcceptanceMismatch => {
                "the accepted plan digest does not equal the exact current plan digest"
            }
            AdapterErrorId::PlanConflict => {
                "the current fit plan contains unresolved ownership conflicts"
            }
            AdapterErrorId::UnsupportedHost => {
                "the required descriptor-bound repository operation is unsupported on this host"
            }
            AdapterErrorId::EffectFailed => {
                "the descriptor-bound local repository effect failed closed"
            }
            AdapterErrorId::ApplyPermitMissing => {
                "the exact root-issued repository-fit apply permit is missing"
            }
            AdapterErrorId::ApplyPermitInvalid => {
                "the repository-fit apply permit does not bind this exact opaque request"
            }
            AdapterErrorId::ApplyPermitExpired => {
                "the repository-fit apply permit is outside its bounded validity window"
            }
            AdapterErrorId::ApplyPermitReplayed => {
                "the repository-fit apply authority was already started or consumed"
            }
            AdapterErrorId::ApplyLeaseInvalid => {
                "the exclusive repository-fit mutation lease is missing or mismatched"
            }
            AdapterErrorId::ApplyMutationScopeViolation => {
                "the repository-fit effect attempted a mutation outside the accepted plan"
            }
            AdapterErrorId::ApplyOutcomeInvalid => {
                "the repository-fit effect outcome did not reconcile to an exact terminal state"
            }
            AdapterErrorId::ApplyRolledBack => {
                "the repository-fit effect failed and the exact prior state was restored"
            }
            AdapterErrorId::ApplyOutcomeAmbiguous => {
                "the repository-fit effect started but exact rollback could not be proved"
            }
        }
    }
}

impl std::fmt::Display for FitAdapterError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.cause())
    }
}

impl std::error::Error for FitAdapterError {}

pub(super) const fn adapter_error(id: AdapterErrorId) -> FitAdapterError {
    FitAdapterError {
        id,
        kernel_error_id: None,
    }
}

pub(super) const fn kernel_error(failure: FitError) -> FitAdapterError {
    let id = match failure.id() {
        FitErrorId::UnsupportedHost => AdapterErrorId::UnsupportedHost,
        FitErrorId::StaleBinding => AdapterErrorId::ContextStale,
        FitErrorId::Conflict => AdapterErrorId::PlanConflict,
        FitErrorId::InvalidSpec | FitErrorId::InvalidPath | FitErrorId::ResourceLimit => {
            AdapterErrorId::InvalidTemplateCatalog
        }
        FitErrorId::UnsafeObject | FitErrorId::ReadFailed => AdapterErrorId::TargetUnavailable,
        FitErrorId::Unauthorized
        | FitErrorId::EffectFailed
        | FitErrorId::RollbackFailed
        | FitErrorId::VerificationFailed => AdapterErrorId::EffectFailed,
    };
    FitAdapterError {
        id,
        kernel_error_id: Some(failure.id()),
    }
}

#[cfg(test)]
#[path = "product_adapter/tests.rs"]
mod tests;
