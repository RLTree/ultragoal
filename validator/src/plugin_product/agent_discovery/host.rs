use super::error::{AgentDiscoveryError, AgentDiscoveryErrorId};
use super::filesystem::{digest, parse_descriptor, safe_relative};
use super::model::{
    AgentAuthorityLayer, AgentLayerObservation, HostFileKind, MAX_CATALOG_BYTES, PLUGIN_NAME,
    PluginManifest, RawAgentRow, RawLayerCatalog,
};
use super::source::SourceAgentCatalog;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub trait HostAgentAuthorityReader {
    fn with_transaction<T>(
        &mut self,
        request: &HostAgentAuthorityRequest,
        operation: impl FnOnce(
            Result<&mut dyn HostAgentAuthorityTransaction, HostAgentAuthorityTransactionError>,
        ) -> T,
    ) -> T;
}

pub trait HostAgentAuthorityTransaction {
    fn provenance_sha256(&self) -> &str;
    fn project_root_sha256(&self) -> &str;
    fn candidate_id(&self) -> &str;
    fn session_id(&self) -> &str;
    fn session_issuance_sha256(&self) -> &str;
    fn observation_nonce_sha256(&self) -> &str;
    fn start_generation(&self) -> u64;
    fn current_generation(&self) -> Result<u64, ()>;
    fn read_catalog(
        &mut self,
        layer: AgentAuthorityLayer,
        maximum: usize,
    ) -> Result<Option<Vec<u8>>, ()>;
    fn enforce_read_only(
        &mut self,
        request: &ReadOnlyEffectRequest,
    ) -> Result<ReadOnlyEffectEnforcement, ()>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostAgentAuthorityTransactionError {
    Unsupported,
    Failed,
}

pub struct HostAgentAuthorityRequest {
    provenance_sha256: String,
    project_root_sha256: String,
    candidate_id: String,
    session_id: String,
    session_issuance_sha256: String,
    observation_nonce_sha256: String,
}

impl HostAgentAuthorityRequest {
    pub fn provenance_sha256(&self) -> &str {
        &self.provenance_sha256
    }

    pub fn project_root_sha256(&self) -> &str {
        &self.project_root_sha256
    }

    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn session_issuance_sha256(&self) -> &str {
        &self.session_issuance_sha256
    }

    pub fn observation_nonce_sha256(&self) -> &str {
        &self.observation_nonce_sha256
    }

    pub(crate) fn issue(
        provenance_sha256: String,
        project_root_sha256: String,
        candidate_id: String,
        session_id: String,
        session_issuance_sha256: String,
        observation_nonce_sha256: String,
    ) -> Self {
        Self {
            provenance_sha256,
            project_root_sha256,
            candidate_id,
            session_id,
            session_issuance_sha256,
            observation_nonce_sha256,
        }
    }
}

pub struct ReadOnlyEffectRequest {
    request_sha256: String,
    binding_sha256: String,
    role_name: String,
    descriptor_sha256: String,
    observation_nonce_sha256: String,
}

impl ReadOnlyEffectRequest {
    pub fn request_sha256(&self) -> &str {
        &self.request_sha256
    }

    pub fn binding_sha256(&self) -> &str {
        &self.binding_sha256
    }

    pub fn role_name(&self) -> &str {
        &self.role_name
    }

    pub fn descriptor_sha256(&self) -> &str {
        &self.descriptor_sha256
    }

    pub fn observation_nonce_sha256(&self) -> &str {
        &self.observation_nonce_sha256
    }

    pub(crate) fn issue(
        binding_sha256: &str,
        role_name: &str,
        descriptor_sha256: &str,
        observation_nonce_sha256: &str,
    ) -> Result<Self, AgentDiscoveryError> {
        let request_sha256 = serde_json::to_vec(&(
            "ReadOnlyEffectRequest-v1",
            binding_sha256,
            role_name,
            descriptor_sha256,
            observation_nonce_sha256,
        ))
        .map(|bytes| digest(&bytes))
        .map_err(|_| invalid_binding())?;
        Ok(Self {
            request_sha256,
            binding_sha256: binding_sha256.to_owned(),
            role_name: role_name.to_owned(),
            descriptor_sha256: descriptor_sha256.to_owned(),
            observation_nonce_sha256: observation_nonce_sha256.to_owned(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ReadOnlyEffectEnforcement {
    request_sha256: String,
    role_name: String,
    sandbox_mode: String,
    workspace_write_denied: bool,
    host_write_denied: bool,
    root_authority_denied: bool,
}

impl ReadOnlyEffectEnforcement {
    pub(crate) fn denied(request: &ReadOnlyEffectRequest) -> Self {
        Self {
            request_sha256: request.request_sha256.clone(),
            role_name: request.role_name.clone(),
            sandbox_mode: "read-only".to_owned(),
            workspace_write_denied: true,
            host_write_denied: true,
            root_authority_denied: true,
        }
    }

    #[cfg(test)]
    pub(crate) fn forged_write_capable(request: &ReadOnlyEffectRequest) -> Self {
        Self {
            request_sha256: request.request_sha256.clone(),
            role_name: request.role_name.clone(),
            sandbox_mode: "workspace-write".to_owned(),
            workspace_write_denied: false,
            host_write_denied: false,
            root_authority_denied: false,
        }
    }

    fn require_exact(&self, request: &ReadOnlyEffectRequest) -> Result<(), AgentDiscoveryError> {
        if self.request_sha256 != request.request_sha256
            || self.role_name != request.role_name
            || self.sandbox_mode != "read-only"
            || !self.workspace_write_denied
            || !self.host_write_denied
            || !self.root_authority_denied
        {
            return Err(AgentDiscoveryError::new(
                AgentDiscoveryErrorId::SandboxPolicyRejected,
            ));
        }
        Ok(())
    }
}

pub(crate) struct BoundHostAgentAuthorityTransaction<'a> {
    transaction: &'a mut dyn HostAgentAuthorityTransaction,
    start_generation: u64,
}

impl<'a> BoundHostAgentAuthorityTransaction<'a> {
    pub(crate) fn bind(
        transaction: &'a mut dyn HostAgentAuthorityTransaction,
        request: &HostAgentAuthorityRequest,
    ) -> Result<Self, AgentDiscoveryError> {
        let bound = Self {
            start_generation: transaction.start_generation(),
            transaction,
        };
        bound.require_current(request)?;
        Ok(bound)
    }

    pub(crate) fn require_current(
        &self,
        request: &HostAgentAuthorityRequest,
    ) -> Result<(), AgentDiscoveryError> {
        if self.transaction.provenance_sha256() != request.provenance_sha256
            || self.transaction.project_root_sha256() != request.project_root_sha256
            || self.transaction.candidate_id() != request.candidate_id
            || self.transaction.session_id() != request.session_id
            || self.transaction.session_issuance_sha256() != request.session_issuance_sha256
            || self.transaction.observation_nonce_sha256() != request.observation_nonce_sha256
            || self.transaction.start_generation() != self.start_generation
            || self
                .transaction
                .current_generation()
                .map_err(|_| unavailable())?
                != self.start_generation
        {
            return Err(changed());
        }
        Ok(())
    }

    pub(crate) fn capture_raw(
        &mut self,
        request: &HostAgentAuthorityRequest,
    ) -> Result<Vec<(AgentAuthorityLayer, Vec<u8>)>, AgentDiscoveryError> {
        self.require_current(request)?;
        let mut result = Vec::with_capacity(AgentAuthorityLayer::ALL.len());
        for layer in AgentAuthorityLayer::ALL {
            self.require_current(request)?;
            let bytes = self
                .transaction
                .read_catalog(layer, MAX_CATALOG_BYTES)
                .map_err(|_| unavailable())?
                .ok_or_else(unavailable)?;
            if bytes.len() > MAX_CATALOG_BYTES {
                return Err(AgentDiscoveryError::new(
                    AgentDiscoveryErrorId::InputTooLarge,
                ));
            }
            result.push((layer, bytes));
        }
        self.require_current(request)?;
        Ok(result)
    }

    pub(crate) fn enforce_effects(
        &mut self,
        source: &SourceAgentCatalog,
        binding_sha256: &str,
        request: &HostAgentAuthorityRequest,
    ) -> Result<String, AgentDiscoveryError> {
        let mut rows = Vec::new();
        for role in source.canonical_agents() {
            self.require_current(request)?;
            let probe = ReadOnlyEffectRequest::issue(
                binding_sha256,
                role.name(),
                role.descriptor_sha256(),
                request.observation_nonce_sha256(),
            )?;
            let enforcement = self
                .transaction
                .enforce_read_only(&probe)
                .map_err(|_| unavailable())?;
            enforcement.require_exact(&probe)?;
            rows.push(enforcement);
        }
        self.require_current(request)?;
        serde_json::to_vec(&("ReadOnlyEffectEnforcementSet-v1", rows))
            .map(|bytes| digest(&bytes))
            .map_err(|_| invalid_binding())
    }
}

pub(crate) fn parse_and_verify_capture(
    raw: &[(AgentAuthorityLayer, Vec<u8>)],
    source: &SourceAgentCatalog,
    request: &HostAgentAuthorityRequest,
) -> Result<Vec<AgentLayerObservation>, AgentDiscoveryError> {
    if raw.len() != AgentAuthorityLayer::ALL.len() {
        return Err(conflict());
    }
    let mut seen_layers = BTreeSet::new();
    let mut observations = Vec::with_capacity(raw.len());
    for (expected_layer, bytes) in raw {
        if !seen_layers.insert(*expected_layer) {
            return Err(conflict());
        }
        let catalog: RawLayerCatalog = serde_json::from_slice(bytes).map_err(|_| conflict())?;
        observations.push(verify_catalog(
            *expected_layer,
            &catalog,
            bytes,
            source,
            request,
        )?);
    }
    if seen_layers != AgentAuthorityLayer::ALL.into_iter().collect() {
        return Err(conflict());
    }
    Ok(observations)
}

fn verify_catalog(
    expected_layer: AgentAuthorityLayer,
    catalog: &RawLayerCatalog,
    raw_bytes: &[u8],
    source: &SourceAgentCatalog,
    request: &HostAgentAuthorityRequest,
) -> Result<AgentLayerObservation, AgentDiscoveryError> {
    if catalog.schema_version != "HostAgentAuthorityCatalog-v1"
        || catalog.layer != expected_layer
        || catalog.plugin_name != PLUGIN_NAME
        || catalog.plugin_version != source.plugin_version()
        || catalog.plugin_manifest_sha256 != source.plugin_manifest_sha256()
        || catalog.plugin_manifest_json.as_bytes() != source.plugin_manifest_bytes()
        || digest(catalog.plugin_manifest_json.as_bytes()) != catalog.plugin_manifest_sha256
        || catalog.project_root_sha256 != request.project_root_sha256
        || catalog.candidate_id != request.candidate_id
        || catalog.session_id != request.session_id
        || catalog.session_issuance_sha256 != request.session_issuance_sha256
        || catalog.observation_nonce_sha256 != request.observation_nonce_sha256
        || !catalog.new_session
    {
        return Err(identity());
    }
    let manifest: PluginManifest =
        serde_json::from_str(&catalog.plugin_manifest_json).map_err(|_| identity())?;
    if manifest.name != PLUGIN_NAME || manifest.version != source.plugin_version() {
        return Err(identity());
    }

    let expected = source
        .canonical_agents()
        .into_iter()
        .map(|row| (row.name().to_owned(), row))
        .collect::<BTreeMap<_, _>>();
    let canonical_normalized = expected
        .keys()
        .map(|name| normalized_name(name))
        .collect::<BTreeSet<_>>();
    let mut names = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut normalized = BTreeSet::new();
    let mut canonical_rows = Vec::new();

    if catalog.agents.len() > 128 {
        return Err(AgentDiscoveryError::new(
            AgentDiscoveryErrorId::InputTooLarge,
        ));
    }
    for row in &catalog.agents {
        require_safe_row(row)?;
        if !names.insert(row.name.clone())
            || !paths.insert(row.manifest_path.clone())
            || !normalized.insert(normalized_name(&row.name))
        {
            return Err(conflict());
        }
        if digest(row.descriptor_toml.as_bytes()) != row.descriptor_sha256 {
            return Err(conflict());
        }
        if harness_agent_authority_namespace(&row.name) {
            return Err(AgentDiscoveryError::new(
                AgentDiscoveryErrorId::LegacyAuthorityActive,
            ));
        }
        let descriptor = parse_descriptor(row.descriptor_toml.as_bytes()).map_err(|error| {
            if error.id() == AgentDiscoveryErrorId::InputTooLarge {
                error
            } else {
                conflict()
            }
        })?;
        if descriptor.name != row.name {
            return Err(conflict());
        }

        if expected_layer == AgentAuthorityLayer::Global {
            let normalized_name = normalized_name(&row.name);
            if canonical_normalized.contains(&normalized_name) {
                return Err(AgentDiscoveryError::new(
                    AgentDiscoveryErrorId::CollidingAuthorityActive,
                ));
            }
            continue;
        }

        let expected_row = expected.get(&row.name).ok_or_else(conflict)?;
        let expected_bytes = source.descriptor_bytes(&row.name).ok_or_else(conflict)?;
        if row.manifest_path != expected_row.manifest_path()
            || row.descriptor_sha256 != expected_row.descriptor_sha256()
            || row.descriptor_toml.as_bytes() != expected_bytes
        {
            return Err(identity());
        }
        if descriptor.sandbox_mode != "read-only" {
            return Err(AgentDiscoveryError::new(
                AgentDiscoveryErrorId::SandboxPolicyRejected,
            ));
        }
        canonical_rows.push(expected_row.clone());
    }

    if expected_layer.requires_exact_canonical_closure() {
        canonical_rows.sort_by(|left, right| left.name().cmp(right.name()));
        let mut expected_rows = expected.into_values().collect::<Vec<_>>();
        expected_rows.sort_by(|left, right| left.name().cmp(right.name()));
        if canonical_rows != expected_rows {
            return Err(conflict());
        }
    }
    Ok(AgentLayerObservation::new(
        expected_layer,
        digest(raw_bytes),
        canonical_rows,
    ))
}

fn require_safe_row(row: &RawAgentRow) -> Result<(), AgentDiscoveryError> {
    if row.name.is_empty()
        || row.name.len() > 80
        || !row.name.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
        || !safe_relative(&row.manifest_path)
        || row.file_kind != HostFileKind::Regular
        || row.link_count != 1
    {
        return Err(AgentDiscoveryError::new(
            AgentDiscoveryErrorId::UnsafeFilesystemEntry,
        ));
    }
    Ok(())
}

fn normalized_name(value: &str) -> String {
    value
        .bytes()
        .filter(|byte| byte.is_ascii_alphanumeric())
        .map(char::from)
        .collect()
}

fn harness_agent_authority_namespace(value: &str) -> bool {
    value
        .strip_prefix("harness")
        .and_then(|suffix| suffix.strip_prefix(['-', '_']))
        .is_some_and(|suffix| {
            !suffix.is_empty()
                && suffix.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'-' | b'_')
                })
        })
}

fn invalid_binding() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::InvalidBinding)
}

fn unavailable() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::ObservationUnavailable)
}

fn changed() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::ObservationChanged)
}

fn conflict() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::ObservationConflict)
}

fn identity() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::IdentityMismatch)
}
