use super::HostAgentAuthorityRequest;
use crate::plugin_manifest;
use crate::plugin_product::agent_discovery::error::{AgentDiscoveryError, AgentDiscoveryErrorId};
use crate::plugin_product::agent_discovery::filesystem::{
    digest, parse_descriptor, safe_relative, valid_sha256,
};
use crate::plugin_product::agent_discovery::model::{
    AgentAuthorityLayer, AgentLayerObservation, HostFileKind, PLUGIN_NAME, PluginManifest,
    RawAgentRow, RawLayerCatalog,
};
use crate::plugin_product::agent_discovery::protocol_codec::decode_catalog;
use crate::plugin_product::agent_discovery::source::SourceAgentCatalog;
use std::collections::{BTreeMap, BTreeSet};

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
        let catalog: RawLayerCatalog = decode_catalog(bytes).map_err(|_| conflict())?;
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
    let roots = observations
        .iter()
        .map(AgentLayerObservation::authority_root_sha256)
        .collect::<BTreeSet<_>>();
    let generations = observations
        .iter()
        .map(AgentLayerObservation::authority_generation_sha256)
        .collect::<BTreeSet<_>>();
    let session_freshness = observations
        .iter()
        .map(AgentLayerObservation::new_session_observed)
        .collect::<BTreeSet<_>>();
    if roots.len() != AgentAuthorityLayer::ALL.len()
        || generations.len() != 1
        || session_freshness.len() != 1
    {
        return Err(identity());
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
        || catalog.project_root_sha256 != request.project_root_sha256()
        || catalog.candidate_id != request.candidate_id()
        || catalog.session_id != request.session_id()
        || catalog.session_issuance_sha256 != request.session_issuance_sha256()
        || catalog.observation_nonce_sha256 != request.observation_nonce_sha256()
        || !valid_sha256(&catalog.authority_root_sha256)
        || !valid_sha256(&catalog.authority_generation_sha256)
        || catalog.transaction_provenance_sha256 != request.provenance_sha256()
    {
        return Err(identity());
    }
    let manifest: PluginManifest = plugin_manifest::parse(
        catalog.plugin_manifest_json.as_bytes(),
        crate::plugin_product::agent_discovery::model::MAX_MANIFEST_BYTES,
    )
    .map_err(|_| identity())?;
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
        if descriptor.sandbox_mode != "read-only" {
            return Err(AgentDiscoveryError::new(
                AgentDiscoveryErrorId::SandboxPolicyRejected,
            ));
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
        catalog.authority_root_sha256.clone(),
        catalog.authority_generation_sha256.clone(),
        catalog.transaction_provenance_sha256.clone(),
        catalog.new_session,
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

fn conflict() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::ObservationConflict)
}

fn identity() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::IdentityMismatch)
}
