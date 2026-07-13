mod apply;
mod error;
mod inspect;
mod local;
mod model;
mod ownership;
mod path;
mod product_adapter;
mod state;

use sha2::{Digest, Sha256};

pub use apply::{apply, rollback, verify};
pub use error::{FitError, FitErrorId};
pub use inspect::{inspect, inspect_with_managed_proofs, plan};
pub use local::LocalRepository;
pub use model::{
    DesiredFile, DesiredState, FitEffects, FitMode, FitReader, Ownership, RepositoryClass,
};
pub use ownership::{ManagedPriorProof, OwnershipProvenance};
pub use path::CanonicalPath;
pub use state::{
    AppliedFit, ExpectedContent, FitCheck, FitConflict, FitInspection, FitPlan, FitVerification,
    Mutation, ObservedDisposition, PlanAuthorization, RollbackPlan,
};

pub(crate) fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub(crate) fn valid_digest(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

pub(crate) use error::error;
#[cfg(unix)]
pub(crate) use local::LocalEffects;
pub(crate) use ownership::issue_managed_prior_proof;
pub(crate) use product_adapter::{
    AdapterErrorId, FitAdapterError, FitApplyPreparationProjection, FitInspectProjection,
    FitPlanRecord, FitVerificationProjection, OpaqueFitApplyRequest, PreparedFitApply,
    inspect_target, plan_target, prepare_apply_request, verify_target,
};
