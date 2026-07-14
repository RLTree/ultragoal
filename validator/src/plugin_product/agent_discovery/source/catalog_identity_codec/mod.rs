use super::*;

#[derive(Serialize)]
struct SourceDigestRow<'a> {
    name: &'a str,
    path: &'a str,
    sha256: &'a str,
}

pub(super) struct SourceCatalogIdentityCodecRequest<'a> {
    project_root_sha256: &'a str,
    candidate_id: &'a str,
    plugin_version: &'a str,
    plugin_manifest_sha256: &'a str,
    agents: &'a [SourceAgentFile],
}

pub(super) struct SourceCatalogIdentityCodecResponse {
    sha256: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SourceCatalogIdentityCodecError {
    Encode,
}

pub(super) fn source_catalog_digest(
    project_root_sha256: &str,
    candidate_id: &str,
    plugin_version: &str,
    plugin_manifest_sha256: &str,
    agents: &[SourceAgentFile],
) -> Result<String, AgentDiscoveryError> {
    execute(SourceCatalogIdentityCodecRequest {
        project_root_sha256,
        candidate_id,
        plugin_version,
        plugin_manifest_sha256,
        agents,
    })
    .map(|response| response.sha256)
    .map_err(|_| invalid_source())
}

fn execute(
    request: SourceCatalogIdentityCodecRequest<'_>,
) -> Result<SourceCatalogIdentityCodecResponse, SourceCatalogIdentityCodecError> {
    let rows = request
        .agents
        .iter()
        .map(|agent| SourceDigestRow {
            name: agent.observation.name(),
            path: agent.observation.manifest_path(),
            sha256: agent.observation.descriptor_sha256(),
        })
        .collect::<Vec<_>>();
    serde_json::to_vec(&(
        "SourceAgentCatalog-v1",
        request.project_root_sha256,
        request.candidate_id,
        request.plugin_version,
        request.plugin_manifest_sha256,
        rows,
    ))
    .map(|bytes| SourceCatalogIdentityCodecResponse {
        sha256: digest(&bytes),
    })
    .map_err(|_| SourceCatalogIdentityCodecError::Encode)
}

pub(super) fn role_file_name(path: &str) -> Result<OsString, AgentDiscoveryError> {
    let path = Path::new(path);
    if path.parent() != Some(Path::new(".codex/agents"))
        || path.extension() != Some(OsStr::new("toml"))
    {
        return Err(invalid_source());
    }
    path.file_name()
        .map(OsStr::to_os_string)
        .ok_or_else(invalid_source)
}

pub(super) fn safe_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
}

pub(super) fn invalid_binding() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::InvalidBinding)
}

pub(super) fn invalid_source() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::InvalidSourceCatalog)
}

pub(super) fn changed() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::ObservationChanged)
}
