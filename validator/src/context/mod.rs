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
#[allow(dead_code)]
mod request;
mod revalidate;
mod types;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod zero_write_tests;

pub use error::ContextError;
pub use request::BuildRequest;
pub use types::{
    CandidateIdentity, CapabilitySet, ConfigurationIdentity, EffectBoundary, EffectClass,
    LiveContext, PermissionIdentity, RootIdentity, SecretSourceIdentity, SelectedInputIdentity,
    ToolCapability,
};
