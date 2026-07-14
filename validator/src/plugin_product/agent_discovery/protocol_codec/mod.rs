use super::filesystem::digest;
use super::host::ReadOnlyEffectEnforcement;
use super::model::{AgentAuthorityLayer, RawLayerCatalog};

pub(super) enum AgentProtocolCodecRequest<'a> {
    SessionIssuance {
        project_root_sha256: &'a str,
        candidate_id: &'a str,
        session_id: &'a str,
        catalog_sha256: &'a str,
        process_id: u32,
        unique: u64,
    },
    ObservationNonce {
        issuance_sha256: &'a str,
        unique: u64,
    },
    SessionBinding {
        project_root_sha256: &'a str,
        candidate_id: &'a str,
        session_id: &'a str,
        catalog_sha256: &'a str,
        plugin_version: &'a str,
        plugin_manifest_sha256: &'a str,
        issuance_sha256: &'a str,
        observation_nonce_sha256: &'a str,
    },
    TransactionProvenance {
        binding_sha256: &'a str,
        project_root_sha256: &'a str,
        candidate_id: &'a str,
        session_id: &'a str,
        issuance_sha256: &'a str,
        observation_nonce_sha256: &'a str,
    },
    VerifiedBinding {
        binding_sha256: &'a str,
        rows: &'a [VerifiedLayerBinding<'a>],
    },
    ReadOnlyEffect {
        binding_sha256: &'a str,
        role_name: &'a str,
        descriptor_sha256: &'a str,
        observation_nonce_sha256: &'a str,
    },
    ReadOnlyEffectSet {
        rows: &'a [ReadOnlyEffectEnforcement],
    },
    DecodeCatalog {
        bytes: &'a [u8],
    },
}

pub(super) enum AgentProtocolCodecResponse {
    Digest(String),
    Catalog(RawLayerCatalog),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AgentProtocolCodecError {
    Encode,
    Decode,
}

pub(super) struct VerifiedLayerBinding<'a> {
    pub(super) layer: AgentAuthorityLayer,
    pub(super) catalog_sha256: &'a str,
    pub(super) authority_root_sha256: &'a str,
    pub(super) authority_generation_sha256: &'a str,
    pub(super) transaction_provenance_sha256: &'a str,
}

impl AgentProtocolCodecResponse {
    pub(super) fn sha256(self) -> String {
        match self {
            Self::Digest(value) => value,
            Self::Catalog(_) => unreachable!("encode request returns digest"),
        }
    }
}

pub(super) fn encode_digest(
    request: AgentProtocolCodecRequest<'_>,
) -> Result<AgentProtocolCodecResponse, AgentProtocolCodecError> {
    execute(request)
}

fn execute(
    request: AgentProtocolCodecRequest<'_>,
) -> Result<AgentProtocolCodecResponse, AgentProtocolCodecError> {
    let bytes = match request {
        AgentProtocolCodecRequest::SessionIssuance {
            project_root_sha256,
            candidate_id,
            session_id,
            catalog_sha256,
            process_id,
            unique,
        } => serde_json::to_vec(&(
            "AgentDiscoverySessionIssuance-v1",
            project_root_sha256,
            candidate_id,
            session_id,
            catalog_sha256,
            process_id,
            unique,
        )),
        AgentProtocolCodecRequest::ObservationNonce {
            issuance_sha256,
            unique,
        } => serde_json::to_vec(&("AgentDiscoveryObservationNonce-v1", issuance_sha256, unique)),
        AgentProtocolCodecRequest::SessionBinding {
            project_root_sha256,
            candidate_id,
            session_id,
            catalog_sha256,
            plugin_version,
            plugin_manifest_sha256,
            issuance_sha256,
            observation_nonce_sha256,
        } => serde_json::to_vec(&(
            "BoundAgentDiscoverySession-v1",
            project_root_sha256,
            candidate_id,
            session_id,
            catalog_sha256,
            plugin_version,
            plugin_manifest_sha256,
            issuance_sha256,
            observation_nonce_sha256,
        )),
        AgentProtocolCodecRequest::TransactionProvenance {
            binding_sha256,
            project_root_sha256,
            candidate_id,
            session_id,
            issuance_sha256,
            observation_nonce_sha256,
        } => serde_json::to_vec(&(
            "HostAgentAuthorityTransaction-v1",
            binding_sha256,
            project_root_sha256,
            candidate_id,
            session_id,
            issuance_sha256,
            observation_nonce_sha256,
        )),
        AgentProtocolCodecRequest::VerifiedBinding {
            binding_sha256,
            rows,
        } => {
            let rows = rows
                .iter()
                .map(|row| {
                    (
                        row.layer,
                        row.catalog_sha256,
                        row.authority_root_sha256,
                        row.authority_generation_sha256,
                        row.transaction_provenance_sha256,
                    )
                })
                .collect::<Vec<_>>();
            serde_json::to_vec(&("VerifiedHostAgentAuthorityBinding-v1", binding_sha256, rows))
        }
        AgentProtocolCodecRequest::ReadOnlyEffect {
            binding_sha256,
            role_name,
            descriptor_sha256,
            observation_nonce_sha256,
        } => serde_json::to_vec(&(
            "ReadOnlyEffectRequest-v1",
            binding_sha256,
            role_name,
            descriptor_sha256,
            observation_nonce_sha256,
        )),
        AgentProtocolCodecRequest::ReadOnlyEffectSet { rows } => {
            serde_json::to_vec(&("ReadOnlyEffectEnforcementSet-v1", rows))
        }
        AgentProtocolCodecRequest::DecodeCatalog { bytes } => {
            return serde_json::from_slice(bytes)
                .map(AgentProtocolCodecResponse::Catalog)
                .map_err(|_| AgentProtocolCodecError::Decode);
        }
    }
    .map_err(|_| AgentProtocolCodecError::Encode)?;
    Ok(AgentProtocolCodecResponse::Digest(digest(&bytes)))
}

pub(super) fn decode_catalog(bytes: &[u8]) -> Result<RawLayerCatalog, AgentProtocolCodecError> {
    match execute(AgentProtocolCodecRequest::DecodeCatalog { bytes })? {
        AgentProtocolCodecResponse::Catalog(catalog) => Ok(catalog),
        AgentProtocolCodecResponse::Digest(_) => Err(AgentProtocolCodecError::Decode),
    }
}
