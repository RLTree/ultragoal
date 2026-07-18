//! Candidate- and session-bound custom-agent authority discovery verifier.
//!
//! This source does not discover or mutate a live Codex host by itself. A
//! separately reviewed supported-host adapter must hold one generation-stable
//! transaction through catalog capture, effect-policy enforcement, and close.

mod error;
#[path = "host_filesystem_adapter/mod.rs"]
mod filesystem;
mod host;
mod local_authority;
mod model;
mod protocol_codec;
mod session;
mod source;
mod supported;

pub(crate) use local_authority::{
    LocalAgentAuthorityObservation, LocalAgentAuthorityRequest, observe_local_authority,
};

#[cfg(all(test, unix))]
pub(crate) use filesystem::{
    ReaddirTestFault, reset_test_io_counts, set_test_readdir_fault, test_io_counts,
    test_readdir_fault_triggered,
};

#[cfg(test)]
pub use error::{AgentDiscoveryError, AgentDiscoveryErrorId};
#[cfg(test)]
pub use host::{
    HostAgentAuthorityReader, HostAgentAuthorityRequest, HostAgentAuthorityTransaction,
    HostAgentAuthorityTransactionError, ReadOnlyEffectEnforcement, ReadOnlyEffectRequest,
};
#[cfg(test)]
pub use model::{
    AgentAuthorityLayer, AgentLayerObservation, AgentRouteEligibility, CanonicalAgentObservation,
    HostFileKind,
};
#[cfg(test)]
pub use session::AgentDiscoverySession;
#[cfg(test)]
pub use source::SourceAgentCatalog;
#[cfg(test)]
pub use supported::{
    SupportedAgentAuthorityFinding, SupportedAgentAuthorityFindingKind,
    SupportedAgentAuthorityObservation, SupportedHostAgentAuthorityReader,
    SupportedHostAgentAuthorityReport, SupportedHostAgentRoots,
};
