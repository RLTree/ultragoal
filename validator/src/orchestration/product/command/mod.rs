//! Read-only command projections over the accepted orchestration product.
//!
//! This module never issues a root permit and never mutates a journal. It
//! produces candidate-bound views and typed requests that root-owned command
//! wiring may independently validate before deciding whether to act.

mod action;
mod diagnose;
mod finding;
mod interrupted;
mod projection;
mod proof;
mod state;

pub use action::{RootActionReason, RootActionRequest};
pub use diagnose::{DiagnosisStatus, OrchestrationDiagnosis, diagnose};
pub use finding::{FindingKind, OrchestrationFinding};
pub use interrupted::{InterruptedRecoveryRequest, InterruptedRecoveryView, inspect_interrupted};
pub use projection::{CommandProjection, project};
pub use proof::{EvidenceDisposition, EvidenceOffer, classify_evidence};
pub use state::{
    NextDisposition, OrchestrationNext, OrchestrationStateRequest, OrchestrationStateView, inspect,
    next,
};
