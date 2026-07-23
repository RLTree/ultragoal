//! Candidate-bound advisory projections for Agentic Engineering.
//!
//! These records consume existing UltraGoal authority and orchestration
//! contracts. They are proposal-only: none can accept work, issue a lease,
//! schedule a retry, promote a claim, or create a second state store.

mod adoption;
mod error;
mod repair;
mod review;
mod selection;
mod selection_lens;
mod selection_model;
mod selection_response;
mod selection_validation;
mod task_evidence;
mod verification;

#[cfg(test)]
mod adoption_tests;
#[cfg(test)]
mod confirmation_tests;
#[cfg(test)]
mod repair_tests;
#[cfg(test)]
mod review_authority_tests;
#[cfg(test)]
mod selection_tests;
#[cfg(test)]
mod tests;

pub use adoption::{
    ADVISORY_ADOPTION_NO_CLAIM, AdvisoryAdoptionDisposition, EngineeringAdvisoryAdoption,
};
pub use error::AdvisoryError;
pub use repair::{
    REPAIR_CIRCUIT_NO_CLAIM, RepairCircuitRoute, SemanticRepairCircuit, SemanticRepairDecision,
    SemanticRepairObservation, decide_semantic_repair,
};
pub use review::{
    MaterialityOutput, REVIEW_VERDICT_NO_CLAIM, ReviewFinding, ReviewFindingSeverity,
    ReviewMaterialityOutput, ReviewVerdict,
};
pub use selection::{ADVISORY_SELECTION_NO_CLAIM, select_advisory};
pub use selection_model::{
    AdvisoryLens, AdvisoryOutcomeClass, AdvisorySelectionDisposition, AdvisorySelectionRequest,
    EngineeringAdvisorySelection,
};
pub use task_evidence::{TASK_EVIDENCE_NO_CLAIM, TaskEvidencePacket};
pub use verification::{
    VerificationModeContract, VerificationModeProposal, VerificationModeProposalInput,
};
