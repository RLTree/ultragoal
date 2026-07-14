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

#[cfg(test)]
pub(crate) use authority::{
    issue_action_permit_for_test, issue_reconcile_permit_for_test, root_authority_for_test,
};
pub use authority::{
    PermitReplayState, PermitTarget, ProductionRootAuthority, RootOperation, RootPermit,
};
pub(crate) use authority::{ProductionExecutionOutcome, ReservationObservation};
#[cfg(test)]
pub(crate) use authority::{RootActionPermitIssuance, RootAuthority, RootReconcilePermitIssuance};
pub use context::{journal_head_identity, ProductContext, ProductWorkspace};
pub(crate) use context::{open_engine, ReadOnlySink};
pub use error::ProductError;
pub use plan::{plan, PlanRequest, ProductPlan};
pub use query::{query, QueryRequest};
pub use reconcile::{ReconcileOutcome, ReconcileRequest};
pub use recover::{RecoverOutcome, RecoverRequest};
pub use resume::{ResumeOutcome, ResumeRequest};
pub use snapshot::{ProductCommitment, ProductSnapshot};
