//! Product boundary over the accepted durable orchestration kernel.
//!
//! Read operations replay the anchored journal without writes. Resume,
//! reconciliation, and interrupted-publication recovery require a short-lived,
//! operation-specific root permit. This module proposes no root integration or
//! completion decision.

mod authority;
pub mod command;
mod context;
mod error;
mod plan;
mod query;
mod reconcile;
mod recover;
mod resume;
pub mod runtime_adapter;
mod snapshot;

pub(crate) use authority::ValidatedExecution;
pub use authority::{
    PermitReplayState, PermitTarget, ProductionRootAuthority, RootOperation, RootPermit,
};
pub(crate) use authority::{
    ReservationObservation, RootActionPermitVerification, RootAuthority,
    RootReconcilePermitVerification,
};
#[cfg(test)]
pub(crate) use authority::{RootActionPermitIssuance, RootReconcilePermitIssuance};
#[cfg(test)]
pub(crate) use authority::{
    issue_action_permit_for_test, issue_reconcile_permit_for_test, root_authority_for_test,
};
pub use context::{ProductContext, ProductWorkspace, journal_head_identity};
pub(crate) use context::{ReadOnlySink, open_engine};
pub use error::ProductError;
pub use plan::{PlanRequest, ProductPlan, plan};
pub use query::{QueryRequest, query};
pub(crate) use reconcile::reconcile;
pub use reconcile::{ReconcileOutcome, ReconcileRequest};
pub(crate) use recover::recover;
pub use recover::{RecoverOutcome, RecoverRequest};
pub(crate) use resume::resume;
pub use resume::{ResumeOutcome, ResumeRequest};
pub use snapshot::{ProductCommitment, ProductSnapshot};
