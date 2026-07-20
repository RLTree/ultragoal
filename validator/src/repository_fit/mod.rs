mod apply;
mod error;
mod inspect;
mod local;
mod local_state;
pub(crate) mod ownership;
mod path;
mod product_adapter;
mod repository_contract;
mod state;

#[cfg(test)]
#[path = "tests/repository_contract.rs"]
mod repository_contract_tests;

use sha2::{Digest, Sha256};

pub use apply::{apply, rollback, verify};
pub use error::{FitError, FitErrorId};
pub use inspect::{inspect, inspect_with_managed_proofs, plan};
pub(crate) use inspect::plan_with_local_state;
pub use local::LocalRepository;
pub use ownership::{ManagedPriorProof, OwnershipProvenance};
pub use path::CanonicalPath;
pub use repository_contract::{
    DesiredFile, DesiredState, FitEffects, FitMode, FitReader, Ownership, RepositoryClass,
};
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
pub(crate) use local_state::{
    LOCAL_STATE_PATH, LocalStateDisposition, LocalStatePlan, inspect_local_state,
};
#[cfg(unix)]
pub(crate) use local::LocalEffects;
#[cfg(test)]
pub(crate) use product_adapter::after_effect_before_terminal_for_test;
pub(crate) use product_adapter::{
    AdapterErrorId, FitAdapterError, PreparedFitApply, RepositoryFitApplyNonce,
    RepositoryFitAuthorityStore, RepositoryFitProductionOutcome, RepositoryFitTrustedClock,
    execute_prepared_apply, inspect_target, plan_target, prepare_apply_request,
    prepare_recovery_intent, recover_prepared_apply, verify_target,
};
