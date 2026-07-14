use super::root_identity_codec::{RootIdentityCodecRequest, encode};
use super::{MAX_HOST_AGENT_ENTRIES, conflict};
use crate::plugin_product::agent_discovery::error::AgentDiscoveryError;
use crate::plugin_product::agent_discovery::filesystem::{
    AnchoredDirectory, AnchoredRoot, SecureFile,
};
use crate::plugin_product::agent_discovery::model::{
    AgentAuthorityLayer, MAX_DESCRIPTOR_BYTES, MAX_MANIFEST_BYTES,
};
use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

/// Explicit roots for one source-local supported-host observation transaction.
#[derive(Clone, Debug)]
pub struct SupportedHostAgentRoots {
    package_root: PathBuf,
    installed_root: PathBuf,
    cache_root: PathBuf,
    global_root: PathBuf,
    project_root: PathBuf,
}

impl SupportedHostAgentRoots {
    pub fn new(
        package_root: impl Into<PathBuf>,
        installed_root: impl Into<PathBuf>,
        cache_root: impl Into<PathBuf>,
        global_root: impl Into<PathBuf>,
        project_root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            package_root: package_root.into(),
            installed_root: installed_root.into(),
            cache_root: cache_root.into(),
            global_root: global_root.into(),
            project_root: project_root.into(),
        }
    }
}

#[derive(Clone)]
pub(super) struct SupportedRootSet {
    package: PluginAuthorityRoot,
    installed: PluginAuthorityRoot,
    cache: PluginAuthorityRoot,
    global: GlobalAuthorityRoot,
    project: PluginAuthorityRoot,
}

impl SupportedRootSet {
    pub(super) fn open(roots: &SupportedHostAgentRoots) -> Result<Self, AgentDiscoveryError> {
        Ok(Self {
            package: PluginAuthorityRoot::open(&roots.package_root)?,
            installed: PluginAuthorityRoot::open(&roots.installed_root)?,
            cache: PluginAuthorityRoot::open(&roots.cache_root)?,
            global: GlobalAuthorityRoot::open(&roots.global_root)?,
            project: PluginAuthorityRoot::open(&roots.project_root)?,
        })
    }

    pub(super) fn revalidate(&self) -> Result<(), AgentDiscoveryError> {
        self.package.revalidate()?;
        self.installed.revalidate()?;
        self.cache.revalidate()?;
        self.global.revalidate()?;
        self.project.revalidate()
    }

    pub(super) fn identity_sha256(&self) -> String {
        let package = self.package.identity_sha256();
        let installed = self.installed.identity_sha256();
        let cache = self.cache.identity_sha256();
        let global = self.global.identity_sha256();
        let project = self.project.identity_sha256();
        encode(RootIdentityCodecRequest::SupportedSet {
            package: &package,
            installed: &installed,
            cache: &cache,
            global: &global,
            project: &project,
        })
        .expect("fixed root identity tuple serializes")
        .sha256()
    }

    pub(super) fn capture(
        &self,
        layer: AgentAuthorityLayer,
    ) -> Result<LayerFiles, AgentDiscoveryError> {
        match layer {
            AgentAuthorityLayer::Package => self.package.capture(),
            AgentAuthorityLayer::Installed => self.installed.capture(),
            AgentAuthorityLayer::Cache => self.cache.capture(),
            AgentAuthorityLayer::Global => self.global.capture(),
            AgentAuthorityLayer::Discovery => self.project.capture(),
        }
    }
}

#[derive(Clone)]
struct PluginAuthorityRoot {
    root: AnchoredRoot,
    agents: AnchoredDirectory,
    plugin: AnchoredDirectory,
}

impl PluginAuthorityRoot {
    fn open(path: &Path) -> Result<Self, AgentDiscoveryError> {
        let root = AnchoredRoot::open(path)?;
        let agents = root.open_dir(".codex/agents")?;
        let plugin = root.open_dir(".codex-plugin")?;
        Ok(Self {
            root,
            agents,
            plugin,
        })
    }

    fn revalidate(&self) -> Result<(), AgentDiscoveryError> {
        self.root.revalidate()?;
        self.root.revalidate_dir(".codex/agents", &self.agents)?;
        self.root.revalidate_dir(".codex-plugin", &self.plugin)
    }

    fn identity_sha256(&self) -> String {
        let root = self.root.identity_sha256();
        let agents = self.agents.identity_sha256();
        let plugin = self.plugin.identity_sha256();
        encode(RootIdentityCodecRequest::PluginRoot {
            root: &root,
            agents: &agents,
            plugin: &plugin,
        })
        .expect("fixed plugin root tuple serializes")
        .sha256()
    }

    fn capture(&self) -> Result<LayerFiles, AgentDiscoveryError> {
        self.revalidate()?;
        let expected_manifest = [OsString::from("plugin.json")]
            .into_iter()
            .collect::<BTreeSet<_>>();
        if !self.plugin.exact_regular_entries(&expected_manifest)? {
            return Err(conflict());
        }
        let manifest = self
            .plugin
            .read_file(OsStr::new("plugin.json"), MAX_MANIFEST_BYTES)?;
        let agents = self
            .agents
            .bounded_regular_files(MAX_HOST_AGENT_ENTRIES, MAX_DESCRIPTOR_BYTES)?;
        self.revalidate()?;
        Ok(LayerFiles::new(
            self.identity_sha256(),
            Some(manifest),
            agents,
        ))
    }
}

#[derive(Clone)]
struct GlobalAuthorityRoot {
    root: AnchoredRoot,
    agents: AnchoredDirectory,
}

impl GlobalAuthorityRoot {
    fn open(path: &Path) -> Result<Self, AgentDiscoveryError> {
        let root = AnchoredRoot::open(path)?;
        let agents = root.open_dir(".codex/agents")?;
        Ok(Self { root, agents })
    }

    fn revalidate(&self) -> Result<(), AgentDiscoveryError> {
        self.root.revalidate()?;
        self.root.revalidate_dir(".codex/agents", &self.agents)
    }

    fn identity_sha256(&self) -> String {
        let root = self.root.identity_sha256();
        let agents = self.agents.identity_sha256();
        encode(RootIdentityCodecRequest::GlobalRoot {
            root: &root,
            agents: &agents,
        })
        .expect("fixed global root tuple serializes")
        .sha256()
    }

    fn capture(&self) -> Result<LayerFiles, AgentDiscoveryError> {
        self.revalidate()?;
        let agents = self
            .agents
            .bounded_regular_files(MAX_HOST_AGENT_ENTRIES, MAX_DESCRIPTOR_BYTES)?;
        self.revalidate()?;
        Ok(LayerFiles::new(self.identity_sha256(), None, agents))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct LayerFiles {
    authority_root_sha256: String,
    manifest: Option<SecureFile>,
    agents: Vec<(OsString, SecureFile)>,
}

impl LayerFiles {
    fn new(
        authority_root_sha256: String,
        manifest: Option<SecureFile>,
        agents: Vec<(OsString, SecureFile)>,
    ) -> Self {
        Self {
            authority_root_sha256,
            manifest,
            agents,
        }
    }

    pub(super) fn authority_root_sha256(&self) -> &str {
        &self.authority_root_sha256
    }

    pub(super) fn manifest(&self) -> Option<&SecureFile> {
        self.manifest.as_ref()
    }

    pub(super) fn agents(&self) -> &[(OsString, SecureFile)] {
        &self.agents
    }

    pub(super) fn content_sha256(&self) -> String {
        let manifest = self.manifest.as_ref().map(|file| file.sha256.as_str());
        let agents = self
            .agents
            .iter()
            .map(|(name, file)| (name.as_encoded_bytes().to_vec(), file.sha256.as_str()))
            .collect::<Vec<_>>();
        encode(RootIdentityCodecRequest::LayerFiles {
            authority_root_sha256: &self.authority_root_sha256,
            manifest_sha256: manifest,
            agents: &agents,
        })
        .expect("fixed layer tuple serializes")
        .sha256()
    }
}
