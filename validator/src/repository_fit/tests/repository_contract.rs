// This contract runs inside `repository_fit` so it can exercise crate-private
// behavior without widening the production API.
mod repository_fit {
    pub(super) use super::super::{
        CanonicalPath, DesiredFile, DesiredState, ExpectedContent, FitEffects, FitError,
        FitErrorId, FitMode, FitPlan, FitReader, LocalRepository, ManagedPriorProof, Ownership,
        OwnershipProvenance, PlanAuthorization, RepositoryClass, apply, digest, error, inspect,
        inspect_with_managed_proofs, ownership, plan, rollback, verify,
    };
}

#[path = "../../../tests/repository_fit_contract/apply_failures.rs"]
mod apply_failures;
#[path = "../../../tests/repository_fit_contract/engine.rs"]
mod engine;
#[path = "../../../tests/repository_fit_contract/enumeration.rs"]
mod enumeration;
#[path = "../../../tests/repository_fit_contract/local_read.rs"]
mod local_read;
#[path = "../../../tests/repository_fit_contract/scenario.rs"]
mod scenario;
#[path = "../../../tests/repository_fit_contract/spec.rs"]
mod spec;
