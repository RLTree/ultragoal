use super::error::{AgentDiscoveryError, AgentDiscoveryErrorId};
use super::filesystem::{
    AnchoredDirectory, AnchoredRoot, SecureFile, digest, parse_descriptor, valid_sha256,
};
use super::model::{
    CanonicalAgentObservation, MAX_DESCRIPTOR_BYTES, MAX_MANIFEST_BYTES, PLUGIN_NAME,
    PluginManifest,
};
use crate::agent_roles::CANONICAL_AGENT_ROLES;
use crate::plugin_manifest;
use serde::Serialize;
use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::path::Path;

const EXACT_AGENT_ENTRY_COUNT: usize = 6;
const EXACT_PLUGIN_ENTRY_COUNT: usize = 1;

#[derive(Clone, Debug)]
struct SourceAgentFile {
    observation: CanonicalAgentObservation,
    file_name: OsString,
    secure_file: SecureFile,
}

#[derive(Clone, Debug)]
pub struct SourceAgentCatalog {
    root: AnchoredRoot,
    codex_root: AnchoredDirectory,
    agents_root: AnchoredDirectory,
    plugin_root: AnchoredDirectory,
    plugin_manifest: SecureFile,
    plugin_version: String,
    project_root_sha256: String,
    candidate_id: String,
    session_id: String,
    catalog_sha256: String,
    agents: Vec<SourceAgentFile>,
}

impl SourceAgentCatalog {
    pub fn capture(
        root: impl AsRef<Path>,
        candidate_id: &str,
        session_id: &str,
    ) -> Result<Self, AgentDiscoveryError> {
        if !valid_sha256(candidate_id) || !valid_sha256(session_id) {
            return Err(invalid_binding());
        }
        let root = AnchoredRoot::open(root.as_ref())?;
        let codex_root = root.open_dir(".codex")?;
        let agents_root = codex_root.open_child(OsStr::new("agents"))?;
        let plugin_root = root.open_dir(".codex-plugin")?;

        let expected_names = CANONICAL_AGENT_ROLES
            .iter()
            .map(|role| role_file_name(role.manifest_path))
            .collect::<Result<BTreeSet<_>, _>>()?;
        if expected_names.len() != EXACT_AGENT_ENTRY_COUNT {
            return Err(invalid_source());
        }
        let expected_plugin_names = [OsString::from("plugin.json")]
            .into_iter()
            .collect::<BTreeSet<_>>();
        if expected_plugin_names.len() != EXACT_PLUGIN_ENTRY_COUNT
            || !agents_root.exact_regular_entries(&expected_names)?
            || !plugin_root.exact_regular_entries(&expected_plugin_names)?
        {
            return Err(invalid_source());
        }

        let mut agents = Vec::with_capacity(CANONICAL_AGENT_ROLES.len());
        for role in CANONICAL_AGENT_ROLES {
            let file_name = role_file_name(role.manifest_path)?;
            let secure_file = agents_root.read_file(&file_name, MAX_DESCRIPTOR_BYTES)?;
            let descriptor = parse_descriptor(&secure_file.bytes)?;
            if descriptor.name != role.name || descriptor.sandbox_mode != "read-only" {
                return Err(invalid_source());
            }
            agents.push(SourceAgentFile {
                observation: CanonicalAgentObservation::new(
                    role.name.to_owned(),
                    role.manifest_path.to_owned(),
                    secure_file.sha256.clone(),
                ),
                file_name,
                secure_file,
            });
        }

        let plugin_manifest =
            plugin_root.read_file(OsStr::new("plugin.json"), MAX_MANIFEST_BYTES)?;
        let parsed: PluginManifest =
            plugin_manifest::parse(&plugin_manifest.bytes, MAX_MANIFEST_BYTES)
                .map_err(|_| invalid_source())?;
        if parsed.name != PLUGIN_NAME
            || !safe_version(&parsed.version)
            || parsed.description.trim().is_empty()
            || parsed.license.as_deref().is_none_or(str::is_empty)
            || parsed.keywords.is_empty()
            || parsed.skills.as_deref() != Some("./skills/")
            || parsed.author.is_none()
            || parsed.interface.is_none()
            || !plugin_manifest::semantic_issues(&parsed).is_empty()
        {
            return Err(invalid_source());
        }
        let project_root_sha256 = digest(root.canonical_path().as_os_str().as_encoded_bytes());
        let catalog_sha256 = source_catalog_digest(
            &project_root_sha256,
            candidate_id,
            &parsed.version,
            &plugin_manifest.sha256,
            &agents,
        )?;
        let catalog = Self {
            root,
            codex_root,
            agents_root,
            plugin_root,
            plugin_manifest,
            plugin_version: parsed.version,
            project_root_sha256,
            candidate_id: candidate_id.to_owned(),
            session_id: session_id.to_owned(),
            catalog_sha256,
            agents,
        };
        catalog.revalidate()?;
        Ok(catalog)
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

    pub fn plugin_version(&self) -> &str {
        &self.plugin_version
    }

    pub fn plugin_manifest_sha256(&self) -> &str {
        &self.plugin_manifest.sha256
    }

    pub fn plugin_manifest_bytes(&self) -> &[u8] {
        &self.plugin_manifest.bytes
    }

    pub fn catalog_sha256(&self) -> &str {
        &self.catalog_sha256
    }

    pub fn canonical_agents(&self) -> Vec<CanonicalAgentObservation> {
        self.agents
            .iter()
            .map(|agent| agent.observation.clone())
            .collect()
    }

    pub(crate) fn descriptor_bytes(&self, name: &str) -> Option<&[u8]> {
        self.agents
            .iter()
            .find(|agent| agent.observation.name() == name)
            .map(|agent| agent.secure_file.bytes.as_slice())
    }
}

#[path = "catalog_identity_codec/mod.rs"]
mod catalog_identity;
mod revalidation;
use catalog_identity::*;
