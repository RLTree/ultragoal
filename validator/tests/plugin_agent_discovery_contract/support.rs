#![allow(dead_code)]

use crate::agent_discovery::{
    AgentAuthorityLayer, HostAgentAuthorityReader, HostAgentAuthorityRequest,
    HostAgentAuthorityTransaction, HostAgentAuthorityTransactionError, ReadOnlyEffectEnforcement,
    ReadOnlyEffectRequest, SourceAgentCatalog, SupportedHostAgentRoots,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(1);
pub const CANDIDATE: &str =
    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
pub const SESSION: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

pub struct TempRepo {
    pub root: PathBuf,
}

impl TempRepo {
    pub fn canonical() -> Self {
        #[cfg(unix)]
        let scratch = PathBuf::from("/tmp");
        #[cfg(not(unix))]
        let scratch = std::env::temp_dir();
        let root = scratch.join(format!(
            "hul-agent-discovery-{}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join(".codex/agents")).unwrap();
        fs::create_dir_all(root.join(".codex-plugin")).unwrap();
        for name in canonical_names() {
            fs::write(
                root.join(format!(".codex/agents/{name}.toml")),
                descriptor(name, "read-only"),
            )
            .unwrap();
        }
        fs::write(
            root.join(".codex-plugin/plugin.json"),
            serde_json::to_vec_pretty(&json!({
                "name": "harness-ultragoal",
                "version": "0.0.11",
                "description": "Fixture plugin product.",
                "author": {"name": "Fixture"},
                "license": "UNLICENSED",
                "keywords": ["fixture"],
                "skills": "./skills/",
                "interface": {"displayName": "Fixture"}
            }))
            .unwrap(),
        )
        .unwrap();
        Self { root }
    }

    pub fn capture(&self) -> SourceAgentCatalog {
        SourceAgentCatalog::capture(&self.root, CANDIDATE, SESSION).unwrap()
    }

    pub fn write(&self, relative: &str, bytes: impl AsRef<[u8]>) {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, bytes).unwrap();
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub struct SupportedHostFixture {
    pub root: PathBuf,
    pub package: PathBuf,
    pub installed: PathBuf,
    pub cache: PathBuf,
    pub global: PathBuf,
    pub project: PathBuf,
}

impl SupportedHostFixture {
    pub fn exact(source: &SourceAgentCatalog) -> Self {
        #[cfg(unix)]
        let scratch = PathBuf::from("/tmp");
        #[cfg(not(unix))]
        let scratch = std::env::temp_dir();
        let root = scratch.join(format!(
            "hul-supported-agent-reader-{}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        let package = root.join("package");
        let installed = root.join("installed");
        let cache = root.join("cache");
        let global = root.join("global");
        let project = root.join("project");
        for plugin_root in [&package, &installed, &cache, &project] {
            write_exact_plugin_root(plugin_root, source);
        }
        fs::create_dir_all(global.join(".codex/agents")).unwrap();
        Self {
            root,
            package,
            installed,
            cache,
            global,
            project,
        }
    }

    pub fn roots(&self) -> SupportedHostAgentRoots {
        SupportedHostAgentRoots::new(
            &self.package,
            &self.installed,
            &self.cache,
            &self.global,
            &self.project,
        )
    }

    pub fn write(&self, root: &Path, relative: &str, bytes: impl AsRef<[u8]>) {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, bytes).unwrap();
    }
}

impl Drop for SupportedHostFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn write_exact_plugin_root(root: &Path, source: &SourceAgentCatalog) {
    fs::create_dir_all(root.join(".codex/agents")).unwrap();
    fs::create_dir_all(root.join(".codex-plugin")).unwrap();
    for name in canonical_names() {
        fs::write(
            root.join(format!(".codex/agents/{name}.toml")),
            source.descriptor_bytes(name).unwrap(),
        )
        .unwrap();
    }
    fs::write(
        root.join(".codex-plugin/plugin.json"),
        source.plugin_manifest_bytes(),
    )
    .unwrap();
}

pub fn canonical_names() -> [&'static str; 6] {
    [
        "claim-falsifier",
        "orchestration-recovery-reviewer",
        "product-journey-reviewer",
        "repo-recon",
        "research-verifier",
        "security-reviewer",
    ]
}

pub fn descriptor(name: &str, sandbox: &str) -> String {
    format!(
        "name = \"{name}\"\ndescription = \"Read-only {name} fixture.\"\ndeveloper_instructions = \"Inspect bounded evidence and do not mutate authority.\"\nsandbox_mode = \"{sandbox}\"\n"
    )
}

pub fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub struct FixtureReader {
    pub source: SourceAgentCatalog,
    pub transaction: Option<FixtureTransaction>,
    configure: Option<Box<dyn FnOnce(&mut FixtureTransaction)>>,
    pub calls: usize,
    pub unavailable: Option<HostAgentAuthorityTransactionError>,
}

impl FixtureReader {
    pub fn exact(source: &SourceAgentCatalog) -> Self {
        Self {
            source: source.clone(),
            transaction: None,
            configure: None,
            calls: 0,
            unavailable: None,
        }
    }

    pub fn configure(&mut self, configure: impl FnOnce(&mut FixtureTransaction) + 'static) {
        self.configure = Some(Box::new(configure));
    }

    pub fn transaction(&self) -> &FixtureTransaction {
        self.transaction.as_ref().expect("transaction was opened")
    }
}

impl HostAgentAuthorityReader for FixtureReader {
    fn with_transaction<T>(
        &mut self,
        request: &HostAgentAuthorityRequest,
        operation: impl FnOnce(
            Result<&mut dyn HostAgentAuthorityTransaction, HostAgentAuthorityTransactionError>,
        ) -> T,
    ) -> T {
        self.calls += 1;
        match self.unavailable {
            Some(error) => operation(Err(error)),
            None => {
                let mut transaction = FixtureTransaction::exact(&self.source, request);
                if let Some(configure) = self.configure.take() {
                    configure(&mut transaction);
                }
                self.transaction = Some(transaction);
                operation(Ok(self.transaction.as_mut().unwrap()))
            }
        }
    }
}

pub struct FixtureTransaction {
    pub provenance: String,
    pub project: String,
    pub candidate: String,
    pub session: String,
    pub issuance: String,
    pub nonce: String,
    pub start_generation: u64,
    pub current_generation: u64,
    pub catalogs: BTreeMap<AgentAuthorityLayer, Vec<u8>>,
    pub second_catalogs: Option<BTreeMap<AgentAuthorityLayer, Vec<u8>>>,
    pub mutate_path_on_read: Option<(usize, PathBuf, Vec<u8>)>,
    pub reads: usize,
    pub probes: usize,
    pub forge_effect: bool,
}

impl FixtureTransaction {
    pub fn exact(source: &SourceAgentCatalog, request: &HostAgentAuthorityRequest) -> Self {
        let catalogs = AgentAuthorityLayer::ALL
            .into_iter()
            .map(|layer| (layer, catalog_bytes(source, request, layer, Vec::new())))
            .collect();
        Self {
            provenance: request.provenance_sha256().to_owned(),
            project: request.project_root_sha256().to_owned(),
            candidate: request.candidate_id().to_owned(),
            session: request.session_id().to_owned(),
            issuance: request.session_issuance_sha256().to_owned(),
            nonce: request.observation_nonce_sha256().to_owned(),
            start_generation: 7,
            current_generation: 7,
            catalogs,
            second_catalogs: None,
            mutate_path_on_read: None,
            reads: 0,
            probes: 0,
            forge_effect: false,
        }
    }

    pub fn mutate_catalog(&mut self, layer: AgentAuthorityLayer, mutate: impl FnOnce(&mut Value)) {
        let mut value: Value = serde_json::from_slice(&self.catalogs[&layer]).unwrap();
        mutate(&mut value);
        self.catalogs
            .insert(layer, serde_json::to_vec(&value).unwrap());
    }

    pub fn add_global_agent(&mut self, name: &str, sandbox: Option<&str>) {
        let descriptor = match sandbox {
            Some(sandbox) => descriptor(name, sandbox),
            None => format!(
                "name = \"{name}\"\ndescription = \"legacy\"\ndeveloper_instructions = \"legacy authority\"\n"
            ),
        };
        self.add_global_agent_bytes(name, &format!("custom-agents/{name}.toml"), descriptor);
    }

    pub fn add_global_agent_bytes(
        &mut self,
        name: &str,
        manifest_path: &str,
        descriptor: impl Into<String>,
    ) {
        let descriptor = descriptor.into();
        let name = name.to_owned();
        let manifest_path = manifest_path.to_owned();
        self.mutate_catalog(AgentAuthorityLayer::Global, |value| {
            value["agents"].as_array_mut().unwrap().push(json!({
                "name": name,
                "manifest_path": manifest_path,
                "descriptor_sha256": digest(descriptor.as_bytes()),
                "descriptor_toml": descriptor,
                "file_kind": "regular",
                "link_count": 1
            }));
        });
    }

    pub fn remove_global_agent(&mut self, name: &str) {
        self.mutate_catalog(AgentAuthorityLayer::Global, |value| {
            value["agents"]
                .as_array_mut()
                .unwrap()
                .retain(|row| row["name"].as_str() != Some(name));
        });
    }
}

impl HostAgentAuthorityTransaction for FixtureTransaction {
    fn provenance_sha256(&self) -> &str {
        &self.provenance
    }
    fn project_root_sha256(&self) -> &str {
        &self.project
    }
    fn candidate_id(&self) -> &str {
        &self.candidate
    }
    fn session_id(&self) -> &str {
        &self.session
    }
    fn session_issuance_sha256(&self) -> &str {
        &self.issuance
    }
    fn observation_nonce_sha256(&self) -> &str {
        &self.nonce
    }
    fn start_generation(&self) -> u64 {
        self.start_generation
    }
    fn current_generation(&self) -> Result<u64, ()> {
        Ok(self.current_generation)
    }
    fn read_catalog(
        &mut self,
        layer: AgentAuthorityLayer,
        _maximum: usize,
    ) -> Result<Option<Vec<u8>>, ()> {
        let round = self.reads / AgentAuthorityLayer::ALL.len();
        self.reads += 1;
        if self
            .mutate_path_on_read
            .as_ref()
            .is_some_and(|(at, _, _)| *at == self.reads)
        {
            let (_, path, bytes) = self.mutate_path_on_read.take().unwrap();
            fs::write(path, bytes).unwrap();
        }
        let catalogs = if round > 0 {
            self.second_catalogs.as_ref().unwrap_or(&self.catalogs)
        } else {
            &self.catalogs
        };
        Ok(catalogs.get(&layer).cloned())
    }
    fn enforce_read_only(
        &mut self,
        request: &ReadOnlyEffectRequest,
    ) -> Result<ReadOnlyEffectEnforcement, ()> {
        self.probes += 1;
        Ok(if self.forge_effect {
            ReadOnlyEffectEnforcement::forged_write_capable(request)
        } else {
            ReadOnlyEffectEnforcement::denied(request)
        })
    }
}

pub fn catalog_bytes(
    source: &SourceAgentCatalog,
    request: &HostAgentAuthorityRequest,
    layer: AgentAuthorityLayer,
    global_agents: Vec<Value>,
) -> Vec<u8> {
    let authority_root_sha256 = digest(format!("fixture-root:{layer:?}").as_bytes());
    let authority_generation_sha256 = digest(b"fixture-generation:7");
    let agents = if layer == AgentAuthorityLayer::Global {
        global_agents
    } else {
        source
            .canonical_agents()
            .into_iter()
            .map(|row| {
                let descriptor =
                    std::str::from_utf8(source.descriptor_bytes(row.name()).expect("descriptor"))
                        .unwrap();
                json!({
                    "name": row.name(),
                    "manifest_path": row.manifest_path(),
                    "descriptor_sha256": row.descriptor_sha256(),
                    "descriptor_toml": descriptor,
                    "file_kind": "regular",
                    "link_count": 1
                })
            })
            .collect()
    };
    serde_json::to_vec(&json!({
        "schema_version": "HostAgentAuthorityCatalog-v1",
        "layer": serde_json::to_value(layer).unwrap(),
        "plugin_name": "harness-ultragoal",
        "plugin_version": source.plugin_version(),
        "plugin_manifest_sha256": source.plugin_manifest_sha256(),
        "plugin_manifest_json": std::str::from_utf8(source.plugin_manifest_bytes()).unwrap(),
        "project_root_sha256": request.project_root_sha256(),
        "candidate_id": request.candidate_id(),
        "session_id": request.session_id(),
        "session_issuance_sha256": request.session_issuance_sha256(),
        "observation_nonce_sha256": request.observation_nonce_sha256(),
        "authority_root_sha256": authority_root_sha256,
        "authority_generation_sha256": authority_generation_sha256,
        "transaction_provenance_sha256": request.provenance_sha256(),
        "new_session": true,
        "agents": agents
    }))
    .unwrap()
}

pub fn replace_source_file(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
}

pub fn tree_snapshot(root: &Path) -> Vec<(String, String)> {
    let mut rows = walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .map(|entry| entry.unwrap())
        .map(|entry| {
            let relative = entry
                .path()
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            let kind = entry.file_type();
            let identity = if kind.is_file() {
                digest(&fs::read(entry.path()).unwrap())
            } else if kind.is_dir() {
                "directory".to_owned()
            } else if kind.is_symlink() {
                format!(
                    "symlink:{}",
                    fs::read_link(entry.path()).unwrap().to_string_lossy()
                )
            } else {
                "special".to_owned()
            };
            (relative, identity)
        })
        .collect::<Vec<_>>();
    rows.sort();
    rows
}
