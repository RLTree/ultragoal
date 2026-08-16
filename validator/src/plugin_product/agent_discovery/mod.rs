//! Candidate- and session-bound custom-agent authority discovery verifier.
//!
//! This source does not discover or mutate a live Codex host by itself. A
//! separately reviewed supported-host adapter must hold one generation-stable
//! transaction through catalog capture, effect-policy enforcement, and close.

mod error;
#[path = "host_filesystem_adapter/mod.rs"]
mod filesystem;
mod host;
mod host_registry_observation;
mod local_authority;
mod model;
mod protocol_codec;
mod session;
mod source;
mod supported;

/// Read-only source-integrity capture used by the current product authority.
///
/// The current-state path needs only the source catalog identity and a
/// revalidation boundary; it must not acquire a host-discovery transaction or
/// any retained inventory authority.
pub(crate) struct CurrentSourceCapture(source::SourceAgentCatalog);

impl CurrentSourceCapture {
    pub(crate) fn catalog_sha256(&self) -> &str {
        self.0.catalog_sha256()
    }

    pub(crate) fn revalidate(&self) -> Result<(), error::AgentDiscoveryError> {
        self.0.revalidate()
    }

    pub(crate) fn plugin_version(&self) -> &str {
        self.0.plugin_version()
    }
}

pub(crate) fn capture_current_source(
    root: &std::path::Path,
    candidate_id: &str,
    session_id: &str,
) -> Result<CurrentSourceCapture, error::AgentDiscoveryError> {
    source::SourceAgentCatalog::capture(root, candidate_id, session_id).map(CurrentSourceCapture)
}

pub(crate) fn capture_installed_source_authority(
    root: &std::path::Path,
    candidate_id: &str,
    session_id: &str,
) -> Result<source::InstalledSourceAuthorityCapture, error::AgentDiscoveryError> {
    source::InstalledSourceAuthorityCapture::capture(root, candidate_id, session_id)
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;

pub(crate) use error::AgentDiscoveryErrorId;
pub(crate) use host_registry_observation::{
    HostPluginRegistryObservation, parse_host_plugin_registry_observation,
    parse_unpinned_host_plugin_registry_observation,
};
pub(crate) use local_authority::{
    AgentRepositoryAdoption, AgentRepositoryAdoptionRequest, adopt_agent_repository,
};

#[cfg(all(test, unix))]
pub(crate) use filesystem::{
    ReaddirTestFault, reset_test_io_counts, set_test_readdir_fault, test_io_counts,
    test_readdir_fault_triggered,
};

#[cfg(test)]
pub use error::AgentDiscoveryError;
#[cfg(test)]
pub use host::{
    HostAgentAuthorityReader, HostAgentAuthorityRequest, HostAgentAuthorityTransaction,
    HostAgentAuthorityTransactionError, ReadOnlyEffectEnforcement, ReadOnlyEffectRequest,
};
#[cfg(test)]
pub use model::AgentAuthorityLayer;
#[cfg(test)]
pub use session::AgentDiscoverySession;
#[cfg(test)]
pub use source::SourceAgentCatalog;
#[cfg(test)]
pub use supported::{
    SupportedAgentAuthorityFindingKind, SupportedHostAgentAuthorityReader, SupportedHostAgentRoots,
};
