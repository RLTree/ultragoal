use crate::plugin_product::agent_discovery::model::AgentAuthorityLayer;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SupportedAgentAuthorityFindingKind {
    MissingCanonical,
    ExtraAuthority,
    LegacyAuthority,
    NormalizedCollision,
    SandboxPolicyMissing,
    WriteCapableSandbox,
    InvalidDescriptor,
    PluginIdentityMismatch,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SupportedAgentAuthorityFinding {
    pub(super) layer: AgentAuthorityLayer,
    pub(super) kind: SupportedAgentAuthorityFindingKind,
    pub(super) authority_name: String,
}

impl SupportedAgentAuthorityFinding {
    pub const fn layer(&self) -> AgentAuthorityLayer {
        self.layer
    }

    pub const fn kind(&self) -> SupportedAgentAuthorityFindingKind {
        self.kind
    }

    pub fn authority_name(&self) -> &str {
        &self.authority_name
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SupportedAgentAuthorityObservation {
    pub(super) layer: AgentAuthorityLayer,
    pub(super) authority_root_sha256: String,
    pub(super) authority_name: String,
    pub(super) manifest_path: String,
    pub(super) descriptor_sha256: String,
    pub(super) sandbox_mode: Option<String>,
}

impl SupportedAgentAuthorityObservation {
    pub const fn layer(&self) -> AgentAuthorityLayer {
        self.layer
    }

    pub fn authority_root_sha256(&self) -> &str {
        &self.authority_root_sha256
    }

    pub fn authority_name(&self) -> &str {
        &self.authority_name
    }

    pub fn sandbox_mode(&self) -> Option<&str> {
        self.sandbox_mode.as_deref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SupportedHostAgentAuthorityReport {
    pub(super) source_catalog_sha256: String,
    pub(super) candidate_id: String,
    pub(super) session_id: String,
    pub(super) transaction_provenance_sha256: String,
    pub(super) generation_sha256: Option<String>,
    pub(super) observations: Vec<SupportedAgentAuthorityObservation>,
    pub(super) findings: Vec<SupportedAgentAuthorityFinding>,
    pub(super) capture_count: usize,
    pub(super) effect_probe_count: usize,
    pub(super) write_operation_count: usize,
    pub(super) failure_code: Option<String>,
}

impl SupportedHostAgentAuthorityReport {
    pub fn transaction_provenance_sha256(&self) -> &str {
        &self.transaction_provenance_sha256
    }

    pub fn generation_sha256(&self) -> Option<&str> {
        self.generation_sha256.as_deref()
    }

    pub fn observations(&self) -> &[SupportedAgentAuthorityObservation] {
        &self.observations
    }

    pub fn findings(&self) -> &[SupportedAgentAuthorityFinding] {
        &self.findings
    }

    pub const fn capture_count(&self) -> usize {
        self.capture_count
    }

    pub const fn effect_probe_count(&self) -> usize {
        self.effect_probe_count
    }

    pub const fn write_operation_count(&self) -> usize {
        self.write_operation_count
    }

    pub fn failure_code(&self) -> Option<&str> {
        self.failure_code.as_deref()
    }
}
