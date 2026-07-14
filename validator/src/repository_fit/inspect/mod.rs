use super::state::ObservedFile;
use super::{
    CanonicalPath, DesiredState, ExpectedContent, FitCheck, FitConflict, FitError, FitErrorId,
    FitInspection, FitMode, FitPlan, FitReader, ManagedPriorProof, Mutation, ObservedDisposition,
    Ownership, OwnershipProvenance, RepositoryClass, RollbackPlan, digest, error, valid_digest,
};
use serde::Serialize;

#[path = "inspection_classification.rs"]
mod inspection_classification;
#[path = "repository_inspection.rs"]
mod repository_inspection;

pub(crate) use inspection_classification::*;
pub use repository_inspection::{inspect, inspect_with_managed_proofs, plan};
