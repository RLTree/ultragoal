//! Pure, candidate-bound live context and structural effect enforcement.
//!
//! Construction performs bounded local read probes only. It does not write receipts,
//! telemetry, caches, Git state, or workspace files, and it never uses the network.

mod authorized_io;
mod bound_context;
mod build;
mod capability;
mod configuration;
mod digest;
mod effects;
mod error;
mod git;
#[cfg(test)]
#[path = "git_tests.rs"]
mod git_tests;
mod inception_subject;
#[cfg(test)]
mod path;
mod planned_write_conflict;
mod process;
#[path = "read/budget.rs"]
mod read_budget;
mod read_observation;
#[path = "read/revalidation.rs"]
mod read_revalidation;
mod read_session;
#[path = "read/snapshot.rs"]
mod read_snapshot;
mod request;
mod revalidate;

#[cfg(test)]
#[path = "read/session_tests.rs"]
mod read_session_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod zero_write_tests;

pub use bound_context::{
    CandidateIdentity, CapabilitySet, ConfigurationIdentity, EffectBoundary, EffectClass,
    LiveContext, PermissionIdentity, RootIdentity, SecretSourceIdentity, SelectedInputIdentity,
    ToolCapability,
};
pub use error::ContextError;
pub(crate) use git::query as query_git;
pub(crate) use inception_subject::identity as inception_subject_identity;
pub(crate) use planned_write_conflict::planned_write_conflict_paths;
pub(crate) use read_session::ReadSession;
pub use request::BuildRequest;
