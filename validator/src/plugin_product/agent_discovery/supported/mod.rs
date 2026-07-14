mod reader;
mod report;
mod root_identity_codec;
mod roots;
mod transaction;

#[cfg(test)]
pub use reader::SupportedHostAgentAuthorityReader;
#[cfg(test)]
pub use report::{
    SupportedAgentAuthorityFinding, SupportedAgentAuthorityFindingKind,
    SupportedAgentAuthorityObservation, SupportedHostAgentAuthorityReport,
};
#[cfg(test)]
pub use roots::SupportedHostAgentRoots;

use super::error::{AgentDiscoveryError, AgentDiscoveryErrorId};

pub(super) const MAX_HOST_AGENT_ENTRIES: usize = 128;

pub(super) fn invalid_binding() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::InvalidBinding)
}

pub(super) fn conflict() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::ObservationConflict)
}

pub(super) fn changed() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::ObservationChanged)
}

pub(super) fn unsafe_entry() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::UnsafeFilesystemEntry)
}
