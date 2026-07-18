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

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;

pub(crate) use confinement::ConfinementPlan;
pub use confinement::ConfinementPolicy;
#[cfg(test)]
pub use confinement::NetworkIsolation;
pub use error::FixtureScheduleError;
#[cfg(test)]
pub use lease::ResourceBinding;
#[cfg(all(test, unix))]
pub(crate) use lease::set_before_capture_hook;
pub use lease::{IsolationLease, LeaseDisposition};
pub(crate) use outcome::{ExecutedFixture, FixtureExecutionRecordCapture};
pub use outcome::{
    ExpectedOutcome, FixtureExecutionBinding, FixtureExecutionRecord, ObservedOutcome,
    OutcomeVerdict,
};
#[cfg(test)]
pub use scheduler::FixtureRun;
pub(crate) use scheduler::{FixtureExecutor, RecordedFixtureExecutor};
pub use scheduler::{FixtureScheduler, RunDisposition};
pub(crate) use spec::stable_digest;
pub use spec::{FixtureKind, FixtureSpec, ResourceKind};
