//! Durable owner-only production issuance and one-use permit execution.
//!
//! Concrete signing, store, and ledger capabilities live below the sealed
//! authority leaf. This module exports only opaque production route types, so
//! adding another child here cannot inherit raw construction authority.

mod sealed_authority;

pub use sealed_authority::{PermitReplayState, ProductionRootAuthority};
pub(crate) use sealed_authority::{ProductionExecutionOutcome, ReservationObservation};

#[cfg(test)]
pub(crate) use sealed_authority::{
    root_authority_for_test, RootActionPermitIssuance, RootAuthority, RootReconcilePermitIssuance,
};
