//! Pure, candidate-bound live context and structural effect enforcement.
//!
//! Construction performs bounded local read probes only. It does not write receipts,
//! telemetry, caches, Git state, or workspace files, and it never uses the network.

#[allow(dead_code)]
mod authorized_io;
mod build;
mod capability;
mod configuration;
mod digest;
mod effects;
mod error;
mod git;
#[allow(dead_code)]
mod path;
mod process;
mod read_budget;
mod read_observation;
mod read_revalidation;
mod read_session;
mod read_snapshot;
#[allow(dead_code)]
mod request;
mod revalidate;
mod types;

#[cfg(test)]
mod read_session_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod zero_write_tests;

pub use error::ContextError;
pub(crate) use read_session::ReadSession;
pub use request::BuildRequest;
pub use types::{
    CandidateIdentity, CapabilitySet, ConfigurationIdentity, EffectBoundary, EffectClass,
    LiveContext, PermissionIdentity, RootIdentity, SecretSourceIdentity, SelectedInputIdentity,
    ToolCapability,
};
