//! Descriptor-confined Darwin host-state transaction candidate.
//!
//! The adapter is deliberately below command and claim authority. It consumes
//! an already-built `PackageSnapshot`, binds every host surface to that exact
//! package and confined root, and mutates only an explicitly supplied
//! `ConfinedRoot`. The seven observable host surfaces remain independent.

mod diagnosis;
mod error;
mod operation;
mod report;
mod surface_codec;
mod surface_state;
mod surfaces;
mod transaction;
mod transaction_codec;
mod transaction_plan;

#[cfg(test)]
#[path = "surface_state/tests.rs"]
mod surface_state_tests;

pub use diagnosis::DarwinHostDiagnosis;
pub use diagnosis::DarwinHostSnapshot;
pub use error::{DarwinHostError, DarwinHostErrorId};
pub use operation::DarwinHostOperation;
pub use report::{DarwinHostTransactionDisposition, DarwinHostTransactionReport};
pub use surface_state::{DarwinSurfaceObservation, DarwinSurfaceStatus};
pub use surfaces::DarwinHostSurface;
pub use transaction::DarwinHostTransactionAdapter;
#[cfg(test)]
pub use transaction::{DarwinTestControl, DarwinTestPoint};
pub use transaction_plan::DarwinHostTransactionPlan;

pub(super) use surface_state::Sha256Digest;

pub(super) const SUPPORTED_PLUGIN_ID: &str = "harness-ultragoal";
pub(super) const SUPPORTED_VERSION: &str = "0.0.12";
pub(super) const SUPPORTED_MARKETPLACE: &str = "local-harness-plugins";
