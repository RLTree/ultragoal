//! Candidate-bound advisory projections for Agentic Engineering.
//!
//! These records consume existing UltraGoal authority and orchestration
//! contracts. They are proposal-only: none can accept work, issue a lease,
//! schedule a retry, promote a claim, or create a second state store.

mod error;
mod repair;
mod review;
mod task_evidence;
mod verification;

#[cfg(test)]
mod tests;

pub use error::AdvisoryError;
pub use repair::{
    REPAIR_CIRCUIT_NO_CLAIM, RepairCircuitRoute, SemanticRepairCircuit, SemanticRepairDecision,
    SemanticRepairObservation, decide_semantic_repair,
};
pub use review::{
    MaterialityOutput, REVIEW_VERDICT_NO_CLAIM, ReviewMaterialityOutput, ReviewVerdict,
};
pub use task_evidence::{TASK_EVIDENCE_NO_CLAIM, TaskEvidencePacket};
pub use verification::{
    VerificationModeContract, VerificationModeProposal, VerificationModeProposalInput,
};
