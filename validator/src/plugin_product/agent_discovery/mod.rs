//! Candidate- and session-bound custom-agent authority discovery verifier.
//!
//! This source does not discover or mutate a live Codex host by itself. A
//! separately reviewed supported-host adapter must hold one generation-stable
//! transaction through catalog capture, effect-policy enforcement, and close.

mod error;
mod filesystem;
mod host;
mod model;
mod session;
mod source;

#[cfg(all(test, unix))]
pub(crate) use filesystem::{
    reset_test_io_counts, set_test_readdir_fault, test_io_counts, test_readdir_fault_triggered,
};

pub use error::{AgentDiscoveryError, AgentDiscoveryErrorId};
pub use host::{
    HostAgentAuthorityReader, HostAgentAuthorityRequest, HostAgentAuthorityTransaction,
    HostAgentAuthorityTransactionError, ReadOnlyEffectEnforcement, ReadOnlyEffectRequest,
};
pub use model::{
    AgentAuthorityLayer, AgentLayerObservation, AgentRouteEligibility, CanonicalAgentObservation,
    HostFileKind,
};
pub use session::AgentDiscoverySession;
pub use source::SourceAgentCatalog;
