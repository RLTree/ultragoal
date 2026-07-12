use super::error::{AgentDiscoveryError, AgentDiscoveryErrorId};
use super::filesystem::{
    AnchoredDirectory, AnchoredRoot, SecureFile, digest, parse_descriptor, valid_sha256,
};
use super::model::{
    CanonicalAgentObservation, MAX_DESCRIPTOR_BYTES, MAX_MANIFEST_BYTES, PLUGIN_NAME,
    PluginManifest,
};
use crate::agent_roles::CANONICAL_AGENT_ROLES;
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
            serde_json::from_slice(&plugin_manifest.bytes).map_err(|_| invalid_source())?;
        if parsed.name != PLUGIN_NAME
            || !safe_version(&parsed.version)
            || parsed.description.trim().is_empty()
            || parsed.license.trim().is_empty()
            || parsed.keywords.is_empty()
            || parsed.skills != "./skills/"
            || parsed.author.is_null()
            || parsed.interface.is_null()
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

    pub(crate) fn revalidate(&self) -> Result<(), AgentDiscoveryError> {
        self.revalidate_with_hooks_internal(|| {}, || {})
    }

    fn revalidate_with_hooks_internal<Before, After>(
        &self,
        before_anchored_reads: Before,
        after_anchored_reads: After,
    ) -> Result<(), AgentDiscoveryError>
    where
        Before: FnOnce(),
        After: FnOnce(),
    {
        self.revalidate_anchors()?;
        before_anchored_reads();
        let content_result = self.revalidate_contents();
        after_anchored_reads();
        let anchor_result = self.revalidate_anchors();
        content_result?;
        anchor_result
    }

    #[cfg(test)]
    pub(crate) fn revalidate_with_test_hooks<Before, After>(
        &self,
        before_anchored_reads: Before,
        after_anchored_reads: After,
    ) -> Result<(), AgentDiscoveryError>
    where
        Before: FnOnce(),
        After: FnOnce(),
    {
        self.revalidate_with_hooks_internal(before_anchored_reads, after_anchored_reads)
    }

    #[cfg(test)]
    pub(crate) fn revalidate_anchored_contents_for_test(&self) -> Result<(), AgentDiscoveryError> {
        self.revalidate_contents()
    }

    fn revalidate_anchors(&self) -> Result<(), AgentDiscoveryError> {
        self.root.revalidate()?;
        self.root.revalidate_dir(".codex", &self.codex_root)?;
        self.root
            .revalidate_dir(".codex/agents", &self.agents_root)?;
        self.root
            .revalidate_dir(".codex-plugin", &self.plugin_root)?;
        Ok(())
    }

    fn revalidate_contents(&self) -> Result<(), AgentDiscoveryError> {
        let expected_names = self
            .agents
            .iter()
            .map(|agent| agent.file_name.clone())
            .collect::<BTreeSet<_>>();
        let expected_plugin_names = [OsString::from("plugin.json")]
            .into_iter()
            .collect::<BTreeSet<_>>();
        if expected_names.len() != EXACT_AGENT_ENTRY_COUNT
            || expected_plugin_names.len() != EXACT_PLUGIN_ENTRY_COUNT
            || !self.agents_root.exact_regular_entries(&expected_names)?
            || !self
                .plugin_root
                .exact_regular_entries(&expected_plugin_names)?
        {
            return Err(changed());
        }
        if !self.plugin_root.same_file(
            OsStr::new("plugin.json"),
            MAX_MANIFEST_BYTES,
            &self.plugin_manifest,
        )? {
            return Err(changed());
        }
        for agent in &self.agents {
            if !self.agents_root.same_file(
                &agent.file_name,
                MAX_DESCRIPTOR_BYTES,
                &agent.secure_file,
            )? {
                return Err(changed());
            }
        }
        Ok(())
    }
}

#[derive(Serialize)]
struct SourceDigestRow<'a> {
    name: &'a str,
    path: &'a str,
    sha256: &'a str,
}

fn source_catalog_digest(
    project_root_sha256: &str,
    candidate_id: &str,
    plugin_version: &str,
    plugin_manifest_sha256: &str,
    agents: &[SourceAgentFile],
) -> Result<String, AgentDiscoveryError> {
    let rows = agents
        .iter()
        .map(|agent| SourceDigestRow {
            name: agent.observation.name(),
            path: agent.observation.manifest_path(),
            sha256: agent.observation.descriptor_sha256(),
        })
        .collect::<Vec<_>>();
    serde_json::to_vec(&(
        "SourceAgentCatalog-v1",
        project_root_sha256,
        candidate_id,
        plugin_version,
        plugin_manifest_sha256,
        rows,
    ))
    .map(|bytes| digest(&bytes))
    .map_err(|_| invalid_source())
}

fn role_file_name(path: &str) -> Result<OsString, AgentDiscoveryError> {
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

fn safe_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
}

fn invalid_binding() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::InvalidBinding)
}

fn invalid_source() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::InvalidSourceCatalog)
}

fn changed() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::ObservationChanged)
}
