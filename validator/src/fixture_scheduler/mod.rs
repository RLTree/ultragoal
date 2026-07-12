//! Isolated, metadata-bound behavioral fixture scheduling primitives.
//!
//! This module owns deterministic ordering, leased resource names, expectation
//! integrity, and causal cleanup/recovery outcomes. A CLI/context adapter is
//! intentionally outside this module's authority.

mod confinement;
mod error;
mod lease;
mod outcome;
mod scheduler;
mod spec;

pub(crate) use confinement::ConfinementPlan;
pub use confinement::{ConfinementPolicy, NetworkIsolation};
pub use error::FixtureScheduleError;
#[cfg(all(test, unix))]
pub(crate) use lease::set_before_capture_hook;
pub use lease::{IsolationLease, LeaseDisposition, ResourceBinding};
pub use outcome::{ExpectedOutcome, ObservedOutcome, OutcomeVerdict};
pub(crate) use scheduler::FixtureExecutor;
pub use scheduler::{FixtureRun, FixtureScheduler, RunDisposition};
pub use spec::{FixtureKind, FixtureSpec, ResourceKind};
