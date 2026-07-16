use super::SupportedTransaction;
use crate::plugin_manifest;
use crate::plugin_product::agent_discovery::error::{AgentDiscoveryError, AgentDiscoveryErrorId};
use crate::plugin_product::agent_discovery::filesystem::{SecureFile, digest};
use crate::plugin_product::agent_discovery::model::{
    AgentAuthorityLayer, HostFileKind, MAX_MANIFEST_BYTES, PLUGIN_NAME, PluginManifest,
    RawAgentRow, RawLayerCatalog,
};
use crate::plugin_product::agent_discovery::source::SourceAgentCatalog;
#[cfg(test)]
use crate::plugin_product::agent_discovery::supported::report::record_layer;
use crate::plugin_product::agent_discovery::supported::roots::{LayerFiles, SupportedRootSet};
use crate::plugin_product::agent_discovery::supported::{invalid_binding, unsafe_entry};
use std::collections::BTreeMap;
use std::ffi::OsStr;

pub(super) enum AgentCatalogCodecRequest<'a> {
    Layer {
        transaction: &'a SupportedTransaction,
        layer: AgentAuthorityLayer,
        files: &'a LayerFiles,
    },
    Generation {
        source: &'a SourceAgentCatalog,
        roots: &'a SupportedRootSet,
        snapshots: &'a BTreeMap<AgentAuthorityLayer, LayerFiles>,
    },
}

pub(super) enum AgentCatalogCodecResponse {
    Bytes(Vec<u8>),
    Digest(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AgentCatalogCodecError {
    Invalid,
    Discovery(AgentDiscoveryErrorId),
}

impl AgentCatalogCodecError {
    fn into_discovery(self) -> AgentDiscoveryError {
        match self {
            Self::Invalid => invalid_binding(),
            Self::Discovery(id) => AgentDiscoveryError::new(id),
        }
    }
}

impl SupportedTransaction {
    pub(super) fn catalog_bytes(
        &self,
        layer: AgentAuthorityLayer,
        files: &LayerFiles,
    ) -> Result<Vec<u8>, AgentDiscoveryError> {
        match execute(AgentCatalogCodecRequest::Layer {
            transaction: self,
            layer,
            files,
        })
        .map_err(AgentCatalogCodecError::into_discovery)?
        {
            AgentCatalogCodecResponse::Bytes(bytes) => Ok(bytes),
            AgentCatalogCodecResponse::Digest(_) => Err(invalid_binding()),
        }
    }
}

fn execute(
    request: AgentCatalogCodecRequest<'_>,
) -> Result<AgentCatalogCodecResponse, AgentCatalogCodecError> {
    match request {
        AgentCatalogCodecRequest::Layer {
            transaction,
            layer,
            files,
        } => encode_layer(transaction, layer, files),
        AgentCatalogCodecRequest::Generation {
            source,
            roots,
            snapshots,
        } => encode_generation(source, roots, snapshots),
    }
}

fn encode_layer(
    transaction: &SupportedTransaction,
    layer: AgentAuthorityLayer,
    files: &LayerFiles,
) -> Result<AgentCatalogCodecResponse, AgentCatalogCodecError> {
    let (manifest_bytes, plugin_name, plugin_version) = match files.manifest() {
        Some(manifest) => {
            let parsed: PluginManifest =
                plugin_manifest::parse(&manifest.bytes, MAX_MANIFEST_BYTES)
                    .map_err(|_| AgentCatalogCodecError::Invalid)?;
            (manifest.bytes.as_slice(), parsed.name, parsed.version)
        }
        None => (
            transaction.source.plugin_manifest_bytes(),
            PLUGIN_NAME.to_owned(),
            transaction.source.plugin_version().to_owned(),
        ),
    };
    let plugin_manifest_json = std::str::from_utf8(manifest_bytes)
        .map_err(|_| AgentCatalogCodecError::Invalid)?
        .to_owned();
    let agents = files
        .agents()
        .iter()
        .map(|(name, file)| raw_agent_row(name, file))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| AgentCatalogCodecError::Discovery(error.id()))?;
    #[cfg(test)]
    record_layer(
        &transaction.report,
        &transaction.source,
        layer,
        files,
        &agents,
        digest(manifest_bytes) == transaction.source.plugin_manifest_sha256(),
    );
    serde_json::to_vec(&RawLayerCatalog {
        schema_version: "HostAgentAuthorityCatalog-v1".to_owned(),
        layer,
        plugin_name,
        plugin_version,
        plugin_manifest_sha256: digest(manifest_bytes),
        plugin_manifest_json,
        project_root_sha256: transaction.project_root_sha256.clone(),
        candidate_id: transaction.candidate_id.clone(),
        session_id: transaction.session_id.clone(),
        session_issuance_sha256: transaction.session_issuance_sha256.clone(),
        observation_nonce_sha256: transaction.observation_nonce_sha256.clone(),
        authority_root_sha256: files.authority_root_sha256().to_owned(),
        authority_generation_sha256: transaction.generation_sha256.clone(),
        transaction_provenance_sha256: transaction.provenance_sha256.clone(),
        new_session: false,
        agents,
    })
    .map(AgentCatalogCodecResponse::Bytes)
    .map_err(|_| AgentCatalogCodecError::Invalid)
}

fn raw_agent_row(file_name: &OsStr, file: &SecureFile) -> Result<RawAgentRow, AgentDiscoveryError> {
    let file_name = file_name.to_str().ok_or_else(unsafe_entry)?;
    let name = file_name
        .strip_suffix(".toml")
        .filter(|name| !name.is_empty())
        .ok_or_else(unsafe_entry)?;
    if !name.bytes().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
    }) {
        return Err(unsafe_entry());
    }
    let descriptor_toml = std::str::from_utf8(&file.bytes)
        .map_err(|_| unsafe_entry())?
        .to_owned();
    Ok(RawAgentRow {
        name: name.to_owned(),
        manifest_path: format!(".codex/agents/{file_name}"),
        descriptor_sha256: file.sha256.clone(),
        descriptor_toml,
        file_kind: HostFileKind::Regular,
        link_count: 1,
    })
}

pub(super) fn generation_sha256(
    source: &SourceAgentCatalog,
    roots: &SupportedRootSet,
    snapshots: &BTreeMap<AgentAuthorityLayer, LayerFiles>,
) -> Result<String, AgentDiscoveryError> {
    match execute(AgentCatalogCodecRequest::Generation {
        source,
        roots,
        snapshots,
    })
    .map_err(|_| invalid_binding())?
    {
        AgentCatalogCodecResponse::Digest(value) => Ok(value),
        AgentCatalogCodecResponse::Bytes(_) => Err(invalid_binding()),
    }
}

fn encode_generation(
    source: &SourceAgentCatalog,
    roots: &SupportedRootSet,
    snapshots: &BTreeMap<AgentAuthorityLayer, LayerFiles>,
) -> Result<AgentCatalogCodecResponse, AgentCatalogCodecError> {
    let rows = AgentAuthorityLayer::ALL
        .into_iter()
        .map(|layer| {
            snapshots
                .get(&layer)
                .map(|files| (layer, files.content_sha256()))
                .ok_or(AgentCatalogCodecError::Invalid)
        })
        .collect::<Result<Vec<_>, _>>()?;
    serde_json::to_vec(&(
        "SupportedHostAgentGeneration-v1",
        source.catalog_sha256(),
        roots.identity_sha256(),
        rows,
    ))
    .map(|bytes| AgentCatalogCodecResponse::Digest(digest(&bytes)))
    .map_err(|_| AgentCatalogCodecError::Invalid)
}
