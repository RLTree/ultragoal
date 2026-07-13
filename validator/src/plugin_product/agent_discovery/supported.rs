use super::error::{AgentDiscoveryError, AgentDiscoveryErrorId};
use super::filesystem::{AnchoredDirectory, AnchoredRoot, SecureFile, digest, parse_descriptor};
use super::host::{
    HostAgentAuthorityReader, HostAgentAuthorityRequest, HostAgentAuthorityTransaction,
    HostAgentAuthorityTransactionError, ReadOnlyEffectEnforcement, ReadOnlyEffectRequest,
};
use super::model::{
    AgentAuthorityLayer, HostFileKind, MAX_DESCRIPTOR_BYTES, MAX_MANIFEST_BYTES, PLUGIN_NAME,
    PluginManifest, RawAgentRow, RawLayerCatalog,
};
use super::source::SourceAgentCatalog;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

const MAX_HOST_AGENT_ENTRIES: usize = 128;

/// Explicit roots for one source-local supported-host observation transaction.
///
/// Every path names a root containing `.codex/agents`. Package, installed,
/// cache, and project roots must also contain `.codex-plugin/plugin.json`.
/// The global root deliberately has no plugin-manifest authority.
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

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SupportedAgentAuthorityFindingKind {
    MissingCanonical,
    ExtraAuthority,
    LegacyAuthority,
    NormalizedCollision,
    SandboxPolicyMissing,
    WriteCapableSandbox,
    InvalidDescriptor,
    PluginIdentityMismatch,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SupportedAgentAuthorityFinding {
    layer: AgentAuthorityLayer,
    kind: SupportedAgentAuthorityFindingKind,
    authority_name: String,
}

impl SupportedAgentAuthorityFinding {
    pub const fn layer(&self) -> AgentAuthorityLayer {
        self.layer
    }

    pub const fn kind(&self) -> SupportedAgentAuthorityFindingKind {
        self.kind
    }

    pub fn authority_name(&self) -> &str {
        &self.authority_name
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SupportedAgentAuthorityObservation {
    layer: AgentAuthorityLayer,
    authority_root_sha256: String,
    authority_name: String,
    manifest_path: String,
    descriptor_sha256: String,
    sandbox_mode: Option<String>,
}

impl SupportedAgentAuthorityObservation {
    pub const fn layer(&self) -> AgentAuthorityLayer {
        self.layer
    }

    pub fn authority_root_sha256(&self) -> &str {
        &self.authority_root_sha256
    }

    pub fn authority_name(&self) -> &str {
        &self.authority_name
    }

    pub fn manifest_path(&self) -> &str {
        &self.manifest_path
    }

    pub fn descriptor_sha256(&self) -> &str {
        &self.descriptor_sha256
    }

    pub fn sandbox_mode(&self) -> Option<&str> {
        self.sandbox_mode.as_deref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SupportedHostAgentAuthorityReport {
    source_catalog_sha256: String,
    candidate_id: String,
    session_id: String,
    transaction_provenance_sha256: String,
    generation_sha256: Option<String>,
    observations: Vec<SupportedAgentAuthorityObservation>,
    findings: Vec<SupportedAgentAuthorityFinding>,
    capture_count: usize,
    effect_probe_count: usize,
    write_operation_count: usize,
    failure_code: Option<String>,
}

impl SupportedHostAgentAuthorityReport {
    pub fn source_catalog_sha256(&self) -> &str {
        &self.source_catalog_sha256
    }

    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn transaction_provenance_sha256(&self) -> &str {
        &self.transaction_provenance_sha256
    }

    pub fn generation_sha256(&self) -> Option<&str> {
        self.generation_sha256.as_deref()
    }

    pub fn observations(&self) -> &[SupportedAgentAuthorityObservation] {
        &self.observations
    }

    pub fn findings(&self) -> &[SupportedAgentAuthorityFinding] {
        &self.findings
    }

    pub const fn capture_count(&self) -> usize {
        self.capture_count
    }

    pub const fn effect_probe_count(&self) -> usize {
        self.effect_probe_count
    }

    pub const fn write_operation_count(&self) -> usize {
        self.write_operation_count
    }

    pub fn failure_code(&self) -> Option<&str> {
        self.failure_code.as_deref()
    }
}

#[derive(Default)]
struct ReportState {
    source_catalog_sha256: String,
    candidate_id: String,
    session_id: String,
    transaction_provenance_sha256: String,
    generation_sha256: Option<String>,
    observations: BTreeSet<SupportedAgentAuthorityObservation>,
    findings: BTreeSet<SupportedAgentAuthorityFinding>,
    capture_count: usize,
    effect_probe_count: usize,
    failure_code: Option<String>,
}

impl ReportState {
    fn snapshot(&self) -> SupportedHostAgentAuthorityReport {
        SupportedHostAgentAuthorityReport {
            source_catalog_sha256: self.source_catalog_sha256.clone(),
            candidate_id: self.candidate_id.clone(),
            session_id: self.session_id.clone(),
            transaction_provenance_sha256: self.transaction_provenance_sha256.clone(),
            generation_sha256: self.generation_sha256.clone(),
            observations: self.observations.iter().cloned().collect(),
            findings: self.findings.iter().cloned().collect(),
            capture_count: self.capture_count,
            effect_probe_count: self.effect_probe_count,
            write_operation_count: 0,
            failure_code: self.failure_code.clone(),
        }
    }
}

/// Descriptor-anchored, read-only host authority reader.
///
/// This adapter never mutates package, install, cache, global, project, or
/// source state. It is intentionally source-local and does not prove that a
/// newly launched Codex process consumed the observed bytes.
pub struct SupportedHostAgentAuthorityReader {
    source: SourceAgentCatalog,
    roots: SupportedRootSet,
    report: Arc<Mutex<ReportState>>,
    #[cfg(test)]
    after_transaction_open: Option<Box<dyn FnOnce()>>,
}

impl SupportedHostAgentAuthorityReader {
    pub fn open(
        source: SourceAgentCatalog,
        roots: SupportedHostAgentRoots,
    ) -> Result<Self, AgentDiscoveryError> {
        source.revalidate()?;
        let roots = SupportedRootSet::open(&roots)?;
        roots.revalidate()?;
        Ok(Self {
            source,
            roots,
            report: Arc::new(Mutex::new(ReportState::default())),
            #[cfg(test)]
            after_transaction_open: None,
        })
    }

    pub fn report(&self) -> SupportedHostAgentAuthorityReport {
        lock_report(&self.report).snapshot()
    }

    #[cfg(test)]
    pub(crate) fn set_after_transaction_open_hook(&mut self, hook: impl FnOnce() + 'static) {
        self.after_transaction_open = Some(Box::new(hook));
    }
}

impl HostAgentAuthorityReader for SupportedHostAgentAuthorityReader {
    fn with_transaction<T>(
        &mut self,
        request: &HostAgentAuthorityRequest,
        operation: impl FnOnce(
            Result<&mut dyn HostAgentAuthorityTransaction, HostAgentAuthorityTransactionError>,
        ) -> T,
    ) -> T {
        reset_report(&self.report, &self.source, request);
        if request.project_root_sha256() != self.source.project_root_sha256()
            || request.candidate_id() != self.source.candidate_id()
            || request.session_id() != self.source.session_id()
        {
            record_failure(&self.report, AgentDiscoveryErrorId::InvalidBinding);
            return operation(Err(HostAgentAuthorityTransactionError::Failed));
        }
        match SupportedTransaction::open(
            self.source.clone(),
            self.roots.clone(),
            request,
            Arc::clone(&self.report),
        ) {
            Ok(mut transaction) => {
                #[cfg(test)]
                if let Some(hook) = self.after_transaction_open.take() {
                    hook();
                }
                operation(Ok(&mut transaction))
            }
            Err(error) => {
                record_failure(&self.report, error.id());
                operation(Err(HostAgentAuthorityTransactionError::Failed))
            }
        }
    }
}

#[derive(Clone)]
struct SupportedRootSet {
    package: PluginAuthorityRoot,
    installed: PluginAuthorityRoot,
    cache: PluginAuthorityRoot,
    global: GlobalAuthorityRoot,
    project: PluginAuthorityRoot,
}

impl SupportedRootSet {
    fn open(roots: &SupportedHostAgentRoots) -> Result<Self, AgentDiscoveryError> {
        Ok(Self {
            package: PluginAuthorityRoot::open(&roots.package_root)?,
            installed: PluginAuthorityRoot::open(&roots.installed_root)?,
            cache: PluginAuthorityRoot::open(&roots.cache_root)?,
            global: GlobalAuthorityRoot::open(&roots.global_root)?,
            project: PluginAuthorityRoot::open(&roots.project_root)?,
        })
    }

    fn revalidate(&self) -> Result<(), AgentDiscoveryError> {
        self.package.revalidate()?;
        self.installed.revalidate()?;
        self.cache.revalidate()?;
        self.global.revalidate()?;
        self.project.revalidate()
    }

    fn identity_sha256(&self) -> String {
        digest(
            serde_json::to_vec(&(
                "SupportedHostAgentRoots-v1",
                self.package.identity_sha256(),
                self.installed.identity_sha256(),
                self.cache.identity_sha256(),
                self.global.identity_sha256(),
                self.project.identity_sha256(),
            ))
            .expect("fixed root identity tuple serializes")
            .as_slice(),
        )
    }

    fn capture(&self, layer: AgentAuthorityLayer) -> Result<LayerFiles, AgentDiscoveryError> {
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
        digest(
            serde_json::to_vec(&(
                "PluginAuthorityRoot-v1",
                self.root.identity_sha256(),
                self.agents.identity_sha256(),
                self.plugin.identity_sha256(),
            ))
            .expect("fixed plugin root tuple serializes")
            .as_slice(),
        )
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
        Ok(LayerFiles {
            authority_root_sha256: self.identity_sha256(),
            manifest: Some(manifest),
            agents,
        })
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
        digest(
            serde_json::to_vec(&(
                "GlobalAuthorityRoot-v1",
                self.root.identity_sha256(),
                self.agents.identity_sha256(),
            ))
            .expect("fixed global root tuple serializes")
            .as_slice(),
        )
    }

    fn capture(&self) -> Result<LayerFiles, AgentDiscoveryError> {
        self.revalidate()?;
        let agents = self
            .agents
            .bounded_regular_files(MAX_HOST_AGENT_ENTRIES, MAX_DESCRIPTOR_BYTES)?;
        self.revalidate()?;
        Ok(LayerFiles {
            authority_root_sha256: self.identity_sha256(),
            manifest: None,
            agents,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LayerFiles {
    authority_root_sha256: String,
    manifest: Option<SecureFile>,
    agents: Vec<(OsString, SecureFile)>,
}

impl LayerFiles {
    fn content_sha256(&self) -> String {
        let manifest = self.manifest.as_ref().map(|file| file.sha256.as_str());
        let agents = self
            .agents
            .iter()
            .map(|(name, file)| (name.as_encoded_bytes(), file.sha256.as_str()))
            .collect::<Vec<_>>();
        digest(
            serde_json::to_vec(&(
                "SupportedAgentLayerFiles-v1",
                &self.authority_root_sha256,
                manifest,
                agents,
            ))
            .expect("fixed layer tuple serializes")
            .as_slice(),
        )
    }
}

struct SupportedTransaction {
    source: SourceAgentCatalog,
    roots: SupportedRootSet,
    snapshots: BTreeMap<AgentAuthorityLayer, LayerFiles>,
    provenance_sha256: String,
    project_root_sha256: String,
    candidate_id: String,
    session_id: String,
    session_issuance_sha256: String,
    observation_nonce_sha256: String,
    generation_sha256: String,
    start_generation: u64,
    report: Arc<Mutex<ReportState>>,
}

impl SupportedTransaction {
    fn open(
        source: SourceAgentCatalog,
        roots: SupportedRootSet,
        request: &HostAgentAuthorityRequest,
        report: Arc<Mutex<ReportState>>,
    ) -> Result<Self, AgentDiscoveryError> {
        source.revalidate()?;
        roots.revalidate()?;
        let mut snapshots = BTreeMap::new();
        for layer in AgentAuthorityLayer::ALL {
            snapshots.insert(layer, roots.capture(layer)?);
        }
        let generation_sha256 = generation_sha256(&source, &roots, &snapshots)?;
        let start_generation =
            u64::from_str_radix(&generation_sha256[7..23], 16).map_err(|_| invalid_binding())?;
        {
            let mut state = lock_report(&report);
            state.generation_sha256 = Some(generation_sha256.clone());
        }
        Ok(Self {
            source,
            roots,
            snapshots,
            provenance_sha256: request.provenance_sha256().to_owned(),
            project_root_sha256: request.project_root_sha256().to_owned(),
            candidate_id: request.candidate_id().to_owned(),
            session_id: request.session_id().to_owned(),
            session_issuance_sha256: request.session_issuance_sha256().to_owned(),
            observation_nonce_sha256: request.observation_nonce_sha256().to_owned(),
            generation_sha256,
            start_generation,
            report,
        })
    }

    fn recapture(&self, layer: AgentAuthorityLayer) -> Result<LayerFiles, AgentDiscoveryError> {
        self.source.revalidate()?;
        let current = self.roots.capture(layer)?;
        if self.snapshots.get(&layer) != Some(&current) {
            return Err(changed());
        }
        self.source.revalidate()?;
        Ok(current)
    }

    fn revalidate_all(&self) -> Result<(), AgentDiscoveryError> {
        self.source.revalidate()?;
        self.roots.revalidate()?;
        for layer in AgentAuthorityLayer::ALL {
            self.recapture(layer)?;
        }
        if generation_sha256(&self.source, &self.roots, &self.snapshots)? != self.generation_sha256
        {
            return Err(changed());
        }
        self.source.revalidate()
    }

    fn catalog_bytes(
        &self,
        layer: AgentAuthorityLayer,
        files: &LayerFiles,
    ) -> Result<Vec<u8>, AgentDiscoveryError> {
        let (manifest_bytes, plugin_name, plugin_version) = match &files.manifest {
            Some(manifest) => {
                let parsed: PluginManifest =
                    serde_json::from_slice(&manifest.bytes).map_err(|_| {
                        AgentDiscoveryError::new(AgentDiscoveryErrorId::IdentityMismatch)
                    })?;
                (manifest.bytes.as_slice(), parsed.name, parsed.version)
            }
            None => (
                self.source.plugin_manifest_bytes(),
                PLUGIN_NAME.to_owned(),
                self.source.plugin_version().to_owned(),
            ),
        };
        let plugin_manifest_json = std::str::from_utf8(manifest_bytes)
            .map_err(|_| AgentDiscoveryError::new(AgentDiscoveryErrorId::IdentityMismatch))?
            .to_owned();
        let agents = files
            .agents
            .iter()
            .map(|(name, file)| raw_agent_row(name, file))
            .collect::<Result<Vec<_>, _>>()?;
        record_layer(
            &self.report,
            &self.source,
            layer,
            files,
            &agents,
            digest(manifest_bytes) == self.source.plugin_manifest_sha256(),
        );
        serde_json::to_vec(&RawLayerCatalog {
            schema_version: "HostAgentAuthorityCatalog-v1".to_owned(),
            layer,
            plugin_name,
            plugin_version,
            plugin_manifest_sha256: digest(manifest_bytes),
            plugin_manifest_json,
            project_root_sha256: self.project_root_sha256.clone(),
            candidate_id: self.candidate_id.clone(),
            session_id: self.session_id.clone(),
            session_issuance_sha256: self.session_issuance_sha256.clone(),
            observation_nonce_sha256: self.observation_nonce_sha256.clone(),
            authority_root_sha256: files.authority_root_sha256.clone(),
            authority_generation_sha256: self.generation_sha256.clone(),
            transaction_provenance_sha256: self.provenance_sha256.clone(),
            // Filesystem observation alone cannot prove that a newly launched
            // Codex session consumed these bytes. A later host/runtime proof
            // must supply that distinct evidence before route eligibility.
            new_session: false,
            agents,
        })
        .map_err(|_| invalid_binding())
    }
}

impl HostAgentAuthorityTransaction for SupportedTransaction {
    fn provenance_sha256(&self) -> &str {
        &self.provenance_sha256
    }

    fn project_root_sha256(&self) -> &str {
        &self.project_root_sha256
    }

    fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    fn session_id(&self) -> &str {
        &self.session_id
    }

    fn session_issuance_sha256(&self) -> &str {
        &self.session_issuance_sha256
    }

    fn observation_nonce_sha256(&self) -> &str {
        &self.observation_nonce_sha256
    }

    fn start_generation(&self) -> u64 {
        self.start_generation
    }

    fn current_generation(&self) -> Result<u64, ()> {
        self.revalidate_all()
            .map(|()| self.start_generation)
            .map_err(|error| {
                record_failure(&self.report, error.id());
            })
    }

    fn read_catalog(
        &mut self,
        layer: AgentAuthorityLayer,
        maximum: usize,
    ) -> Result<Option<Vec<u8>>, ()> {
        let result = (|| {
            let files = self.recapture(layer)?;
            let bytes = self.catalog_bytes(layer, &files)?;
            if bytes.len() > maximum {
                return Err(AgentDiscoveryError::new(
                    AgentDiscoveryErrorId::InputTooLarge,
                ));
            }
            lock_report(&self.report).capture_count += 1;
            Ok(Some(bytes))
        })();
        result.map_err(|error| {
            record_failure(&self.report, error.id());
        })
    }

    fn enforce_read_only(
        &mut self,
        request: &ReadOnlyEffectRequest,
    ) -> Result<ReadOnlyEffectEnforcement, ()> {
        let result = (|| {
            if request.observation_nonce_sha256() != self.observation_nonce_sha256 {
                return Err(invalid_binding());
            }
            self.source.revalidate()?;
            let source_bytes = self
                .source
                .descriptor_bytes(request.role_name())
                .ok_or_else(conflict)?;
            if digest(source_bytes) != request.descriptor_sha256()
                || parse_descriptor(source_bytes)?.sandbox_mode != "read-only"
            {
                return Err(AgentDiscoveryError::new(
                    AgentDiscoveryErrorId::SandboxPolicyRejected,
                ));
            }
            for layer in [
                AgentAuthorityLayer::Package,
                AgentAuthorityLayer::Installed,
                AgentAuthorityLayer::Cache,
                AgentAuthorityLayer::Discovery,
            ] {
                let files = self.recapture(layer)?;
                let file = find_agent_file(&files, request.role_name()).ok_or_else(conflict)?;
                if file.sha256 != request.descriptor_sha256()
                    || parse_descriptor(&file.bytes)?.sandbox_mode != "read-only"
                {
                    return Err(AgentDiscoveryError::new(
                        AgentDiscoveryErrorId::SandboxPolicyRejected,
                    ));
                }
            }
            self.revalidate_all()?;
            lock_report(&self.report).effect_probe_count += 1;
            Ok(ReadOnlyEffectEnforcement::denied(request))
        })();
        result.map_err(|error| {
            record_failure(&self.report, error.id());
        })
    }
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

fn find_agent_file<'a>(files: &'a LayerFiles, name: &str) -> Option<&'a SecureFile> {
    let expected = format!("{name}.toml");
    files
        .agents
        .iter()
        .find(|(file_name, _)| file_name == OsStr::new(&expected))
        .map(|(_, file)| file)
}

fn generation_sha256(
    source: &SourceAgentCatalog,
    roots: &SupportedRootSet,
    snapshots: &BTreeMap<AgentAuthorityLayer, LayerFiles>,
) -> Result<String, AgentDiscoveryError> {
    let rows = AgentAuthorityLayer::ALL
        .into_iter()
        .map(|layer| {
            snapshots
                .get(&layer)
                .map(|files| (layer, files.content_sha256()))
                .ok_or_else(invalid_binding)
        })
        .collect::<Result<Vec<_>, _>>()?;
    serde_json::to_vec(&(
        "SupportedHostAgentGeneration-v1",
        source.catalog_sha256(),
        roots.identity_sha256(),
        rows,
    ))
    .map(|bytes| digest(&bytes))
    .map_err(|_| invalid_binding())
}

fn record_layer(
    report: &Arc<Mutex<ReportState>>,
    source: &SourceAgentCatalog,
    layer: AgentAuthorityLayer,
    files: &LayerFiles,
    agents: &[RawAgentRow],
    manifest_matches: bool,
) {
    let canonical = source
        .canonical_agents()
        .into_iter()
        .map(|row| row.name().to_owned())
        .collect::<BTreeSet<_>>();
    let canonical_normalized = canonical
        .iter()
        .map(|name| normalized_name(name))
        .collect::<BTreeSet<_>>();
    let observed = agents
        .iter()
        .map(|row| row.name.clone())
        .collect::<BTreeSet<_>>();
    let mut normalized_seen = BTreeSet::new();
    let mut state = lock_report(report);
    if files.manifest.is_some() && !manifest_matches {
        state.findings.insert(finding(
            layer,
            SupportedAgentAuthorityFindingKind::PluginIdentityMismatch,
            PLUGIN_NAME,
        ));
    }
    for row in agents {
        let sandbox_mode = toml::from_str::<toml::Value>(&row.descriptor_toml)
            .ok()
            .and_then(|value| {
                value
                    .get("sandbox_mode")
                    .and_then(toml::Value::as_str)
                    .map(str::to_owned)
            });
        state
            .observations
            .insert(SupportedAgentAuthorityObservation {
                layer,
                authority_root_sha256: files.authority_root_sha256.clone(),
                authority_name: row.name.clone(),
                manifest_path: row.manifest_path.clone(),
                descriptor_sha256: row.descriptor_sha256.clone(),
                sandbox_mode: sandbox_mode.clone(),
            });
        let normalized = normalized_name(&row.name);
        if !normalized_seen.insert(normalized.clone()) {
            state.findings.insert(finding(
                layer,
                SupportedAgentAuthorityFindingKind::NormalizedCollision,
                &row.name,
            ));
        }
        if harness_agent_authority_namespace(&row.name) {
            state.findings.insert(finding(
                layer,
                SupportedAgentAuthorityFindingKind::LegacyAuthority,
                &row.name,
            ));
        } else if layer == AgentAuthorityLayer::Global && canonical_normalized.contains(&normalized)
        {
            state.findings.insert(finding(
                layer,
                SupportedAgentAuthorityFindingKind::NormalizedCollision,
                &row.name,
            ));
        } else if layer != AgentAuthorityLayer::Global && !canonical.contains(&row.name) {
            state.findings.insert(finding(
                layer,
                SupportedAgentAuthorityFindingKind::ExtraAuthority,
                &row.name,
            ));
        }
        match sandbox_mode.as_deref() {
            None => {
                state.findings.insert(finding(
                    layer,
                    SupportedAgentAuthorityFindingKind::SandboxPolicyMissing,
                    &row.name,
                ));
                state.findings.insert(finding(
                    layer,
                    SupportedAgentAuthorityFindingKind::InvalidDescriptor,
                    &row.name,
                ));
            }
            Some("read-only") => {}
            Some(_) => {
                state.findings.insert(finding(
                    layer,
                    SupportedAgentAuthorityFindingKind::WriteCapableSandbox,
                    &row.name,
                ));
            }
        }
    }
    if layer != AgentAuthorityLayer::Global {
        for name in canonical.difference(&observed) {
            state.findings.insert(finding(
                layer,
                SupportedAgentAuthorityFindingKind::MissingCanonical,
                name,
            ));
        }
    }
}

fn finding(
    layer: AgentAuthorityLayer,
    kind: SupportedAgentAuthorityFindingKind,
    authority_name: &str,
) -> SupportedAgentAuthorityFinding {
    SupportedAgentAuthorityFinding {
        layer,
        kind,
        authority_name: authority_name.to_owned(),
    }
}

fn reset_report(
    report: &Arc<Mutex<ReportState>>,
    source: &SourceAgentCatalog,
    request: &HostAgentAuthorityRequest,
) {
    *lock_report(report) = ReportState {
        source_catalog_sha256: source.catalog_sha256().to_owned(),
        candidate_id: request.candidate_id().to_owned(),
        session_id: request.session_id().to_owned(),
        transaction_provenance_sha256: request.provenance_sha256().to_owned(),
        ..ReportState::default()
    };
}

fn record_failure(report: &Arc<Mutex<ReportState>>, id: AgentDiscoveryErrorId) {
    let mut report = lock_report(report);
    if report.failure_code.is_none() {
        report.failure_code = Some(id.code().to_owned());
    }
}

fn lock_report(report: &Arc<Mutex<ReportState>>) -> MutexGuard<'_, ReportState> {
    report
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
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

fn conflict() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::ObservationConflict)
}

fn changed() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::ObservationChanged)
}

fn unsafe_entry() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::UnsafeFilesystemEntry)
}
