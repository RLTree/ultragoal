use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum AgentDiscoveryErrorId {
    InvalidBinding,
    InvalidSourceCatalog,
    UnsafeFilesystemEntry,
    InputTooLarge,
    ObservationUnavailable,
    ObservationChanged,
    ObservationConflict,
    IdentityMismatch,
    LegacyAuthorityActive,
    CollidingAuthorityActive,
    SandboxPolicyRejected,
    SessionStateRejected,
    SessionReplay,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AgentDiscoveryError {
    id: AgentDiscoveryErrorId,
}

impl AgentDiscoveryError {
    pub(crate) const fn new(id: AgentDiscoveryErrorId) -> Self {
        Self { id }
    }

    pub const fn id(self) -> AgentDiscoveryErrorId {
        self.id
    }
}

impl fmt::Display for AgentDiscoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self.id {
            AgentDiscoveryErrorId::InvalidBinding => "agent discovery binding is invalid",
            AgentDiscoveryErrorId::InvalidSourceCatalog => {
                "canonical agent source catalog is invalid"
            }
            AgentDiscoveryErrorId::UnsafeFilesystemEntry => {
                "agent discovery encountered an unsafe filesystem entry"
            }
            AgentDiscoveryErrorId::InputTooLarge => "agent discovery input exceeds its bound",
            AgentDiscoveryErrorId::ObservationUnavailable => {
                "required host agent observation is unavailable"
            }
            AgentDiscoveryErrorId::ObservationChanged => {
                "host agent observation changed during capture"
            }
            AgentDiscoveryErrorId::ObservationConflict => {
                "host agent observation contains conflicting authority"
            }
            AgentDiscoveryErrorId::IdentityMismatch => {
                "host agent observation does not match the bound candidate"
            }
            AgentDiscoveryErrorId::LegacyAuthorityActive => "legacy agent authority remains active",
            AgentDiscoveryErrorId::CollidingAuthorityActive => {
                "colliding agent authority remains active"
            }
            AgentDiscoveryErrorId::SandboxPolicyRejected => {
                "agent sandbox or effect policy is not read-only"
            }
            AgentDiscoveryErrorId::SessionStateRejected => {
                "agent discovery session rejected the operation"
            }
            AgentDiscoveryErrorId::SessionReplay => "agent discovery session was already consumed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for AgentDiscoveryError {}
