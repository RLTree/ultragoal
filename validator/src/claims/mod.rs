//! Claim-authority model. Adoption is deliberately independent from runtime wiring.

mod coverage;
mod decision;
mod definition;
mod evidence;
mod false_pass;
mod false_pass_guard;
mod false_pass_integrity;
mod false_pass_receipt;
mod guard;

pub use decision::{ClaimDecision, DecisionLedger, DecisionStatus, Projection};
pub use definition::{ClaimDefinition, ClaimDefinitions};
pub use evidence::{
    Actor, ActorRole, ClaimObligation, EvidenceEnvelope, EvidenceKind, ObligationKind,
    ObligationResult, Observation,
};
pub use false_pass::{
    LocalNegativeControlAuthority, SemanticControlModel, SemanticControlObservation,
};
#[cfg(test)]
pub(crate) use false_pass::{
    PRIVATE_TRANSPORT_UNAVAILABLE, TestSubstitution, product_executor_catalog_preflight_for_test,
};
pub use guard::ClaimGuard;
