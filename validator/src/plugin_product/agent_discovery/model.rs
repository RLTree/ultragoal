use serde::{Deserialize, Serialize};

pub(crate) const MAX_DESCRIPTOR_BYTES: usize = 64 * 1024;
pub(crate) const MAX_CATALOG_BYTES: usize = 2 * 1024 * 1024;
pub(crate) const MAX_MANIFEST_BYTES: usize = 256 * 1024;
pub(crate) const PLUGIN_NAME: &str = "harness-ultragoal";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentAuthorityLayer {
    Package,
    Installed,
    Cache,
    Global,
    Discovery,
}

impl AgentAuthorityLayer {
    pub const ALL: [Self; 5] = [
        Self::Package,
        Self::Installed,
        Self::Cache,
        Self::Global,
        Self::Discovery,
    ];

    pub(crate) const fn requires_exact_canonical_closure(self) -> bool {
        !matches!(self, Self::Global)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HostFileKind {
    Regular,
    Symlink,
    Directory,
    Fifo,
    Socket,
    Device,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CanonicalAgentObservation {
    name: String,
    manifest_path: String,
    descriptor_sha256: String,
}

impl CanonicalAgentObservation {
    pub(crate) fn new(name: String, manifest_path: String, descriptor_sha256: String) -> Self {
        Self {
            name,
            manifest_path,
            descriptor_sha256,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn manifest_path(&self) -> &str {
        &self.manifest_path
    }

    pub fn descriptor_sha256(&self) -> &str {
        &self.descriptor_sha256
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AgentLayerObservation {
    layer: AgentAuthorityLayer,
    catalog_sha256: String,
    authority_root_sha256: String,
    authority_generation_sha256: String,
    transaction_provenance_sha256: String,
    new_session_observed: bool,
    canonical_agents: Vec<CanonicalAgentObservation>,
}

impl AgentLayerObservation {
    pub(crate) fn new(
        layer: AgentAuthorityLayer,
        catalog_sha256: String,
        authority_root_sha256: String,
        authority_generation_sha256: String,
        transaction_provenance_sha256: String,
        new_session_observed: bool,
        canonical_agents: Vec<CanonicalAgentObservation>,
    ) -> Self {
        Self {
            layer,
            catalog_sha256,
            authority_root_sha256,
            authority_generation_sha256,
            transaction_provenance_sha256,
            new_session_observed,
            canonical_agents,
        }
    }

    pub const fn layer(&self) -> AgentAuthorityLayer {
        self.layer
    }

    pub fn catalog_sha256(&self) -> &str {
        &self.catalog_sha256
    }

    pub fn authority_root_sha256(&self) -> &str {
        &self.authority_root_sha256
    }

    pub fn authority_generation_sha256(&self) -> &str {
        &self.authority_generation_sha256
    }

    pub fn transaction_provenance_sha256(&self) -> &str {
        &self.transaction_provenance_sha256
    }

    pub const fn new_session_observed(&self) -> bool {
        self.new_session_observed
    }

    pub fn canonical_agents(&self) -> &[CanonicalAgentObservation] {
        &self.canonical_agents
    }
}

#[derive(Debug, Serialize)]
pub struct AgentRouteEligibility {
    binding_sha256: String,
    source_catalog_sha256: String,
    package_version: String,
    layers: Vec<AgentLayerObservation>,
    sandbox_effect_sha256: String,
    new_session_observed: bool,
    route_eligible: bool,
    claim_effect: bool,
}

impl AgentRouteEligibility {
    pub(crate) fn observed(
        binding_sha256: String,
        source_catalog_sha256: String,
        package_version: String,
        layers: Vec<AgentLayerObservation>,
        sandbox_effect_sha256: String,
        new_session_observed: bool,
    ) -> Self {
        Self {
            binding_sha256,
            source_catalog_sha256,
            package_version,
            layers,
            sandbox_effect_sha256,
            new_session_observed,
            route_eligible: new_session_observed,
            claim_effect: false,
        }
    }

    pub fn binding_sha256(&self) -> &str {
        &self.binding_sha256
    }

    pub fn source_catalog_sha256(&self) -> &str {
        &self.source_catalog_sha256
    }

    pub fn package_version(&self) -> &str {
        &self.package_version
    }

    pub fn layers(&self) -> &[AgentLayerObservation] {
        &self.layers
    }

    pub fn sandbox_effect_sha256(&self) -> &str {
        &self.sandbox_effect_sha256
    }

    pub const fn new_session_observed(&self) -> bool {
        self.new_session_observed
    }

    pub const fn route_eligible(&self) -> bool {
        self.route_eligible
    }

    pub const fn has_claim_effect(&self) -> bool {
        self.claim_effect
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProjectAgentDescriptor {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) developer_instructions: String,
    pub(crate) sandbox_mode: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PluginManifest {
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) description: String,
    pub(crate) author: serde_json::Value,
    pub(crate) license: String,
    pub(crate) keywords: Vec<String>,
    pub(crate) skills: String,
    pub(crate) interface: serde_json::Value,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawAgentRow {
    pub(crate) name: String,
    pub(crate) manifest_path: String,
    pub(crate) descriptor_sha256: String,
    pub(crate) descriptor_toml: String,
    pub(crate) file_kind: HostFileKind,
    pub(crate) link_count: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawLayerCatalog {
    pub(crate) schema_version: String,
    pub(crate) layer: AgentAuthorityLayer,
    pub(crate) plugin_name: String,
    pub(crate) plugin_version: String,
    pub(crate) plugin_manifest_sha256: String,
    pub(crate) plugin_manifest_json: String,
    pub(crate) project_root_sha256: String,
    pub(crate) candidate_id: String,
    pub(crate) session_id: String,
    pub(crate) session_issuance_sha256: String,
    pub(crate) observation_nonce_sha256: String,
    pub(crate) authority_root_sha256: String,
    pub(crate) authority_generation_sha256: String,
    pub(crate) transaction_provenance_sha256: String,
    pub(crate) new_session: bool,
    pub(crate) agents: Vec<RawAgentRow>,
}
