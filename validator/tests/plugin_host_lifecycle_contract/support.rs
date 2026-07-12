use crate::distribution::{
    CodexPlugin, ConfinedRoot, HostCapabilityDeclaration, HostCommandPlan, JourneyBinding,
    MarketplacePlan, PackagePlan, PackageSnapshot, RuntimeObservation, RuntimeProbePlan,
    ScopedFile, build_package, execute_runtime_probe, plan_codex_marketplace, plan_package,
    registry_document,
};
use crate::host_lifecycle::{
    HostLifecycleSession, HostObservationTransactionRequest, HostScopeAuthority, HostSurfaceReader,
    HostSurfaceTransaction, HostSurfaceTransactionError,
};
use crate::plugin_product::lifecycle::{
    LifecycleAuthorization, LifecycleIntent, LifecyclePlan, LifecycleRequest, LifecycleState,
    PackageAuthority, Version, plan,
};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

pub const CONTEXT: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
pub const CANDIDATE: &str =
    "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
static NEXT: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub struct Bundle {
    pub plan: PackagePlan,
    pub snapshot: PackageSnapshot,
    pub authority: PackageAuthority,
}

pub struct Fixture {
    pub root: PathBuf,
    pub project: PathBuf,
}

impl Fixture {
    pub fn new(label: &str) -> Self {
        let root = PathBuf::from("/tmp").join(format!(
            "hul-distribution-plugin-host-lifecycle-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let project = root.join("project");
        fs::create_dir_all(&project).unwrap();
        Self { root, project }
    }

    pub fn confined(&self) -> ConfinedRoot {
        ConfinedRoot::open(&self.root).unwrap()
    }

    pub fn bundle(&self, version: &str) -> Bundle {
        let label = version.replace('.', "-");
        let source = self.root.join(format!("source-{label}"));
        fs::create_dir_all(&source).unwrap();
        let manifest = json!({
            "name":"harness-ultragoal", "version":version,
            "description":"Repository fit, routine work, diagnosis, proof, and migration.",
            "author":{"name":"Terry Noblin","email":"tree@terrynoblin.dev","url":"https://terrynoblin.dev"},
            "homepage":"https://terrynoblin.dev/harness-ultragoal",
            "repository":"https://github.com/terrynoblin/harness-ultragoal",
            "license":"UNLICENSED", "keywords":["agent-first","verification"],
            "skills":"./skills/",
            "interface":{
                "displayName":"Harness Ultragoal", "shortDescription":"One evidence-bound front door.",
                "longDescription":"Route repository work through explicit authority and effects.",
                "developerName":"Terry Noblin", "category":"Productivity",
                "capabilities":["Read","Write"], "websiteURL":"https://terrynoblin.dev/harness-ultragoal",
                "defaultPrompt":["Classify this repository task and route it safely."],
                "brandColor":"#3B82F6"
            }
        });
        fs::write(
            source.join("plugin.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        fs::write(
            source.join("front.md"),
            b"---\nname: harness-ultragoal\ndescription: Harness front door\n---\n",
        )
        .unwrap();
        let spec = json!({
            "schema":"harness-ultragoal.package-plan.v1", "context_id":CONTEXT,
            "candidate_id":CANDIDATE, "plugin_id":"harness-ultragoal", "version":version,
            "source_date_epoch":1_700_000_000u64,
            "entries":[
                {"path":".codex-plugin/plugin.json","source_path":format!("source-{label}/plugin.json"),"role":"manifest","executable":false},
                {"path":"skills/harness-ultragoal/SKILL.md","source_path":format!("source-{label}/front.md"),"role":"skill","executable":false}
            ]
        });
        let plan = plan_package(&self.root, &serde_json::to_vec(&spec).unwrap()).unwrap();
        let mut sink = ScopedFile::new(
            self.confined(),
            &format!("packages/harness-ultragoal-{label}.hugpkg"),
        )
        .unwrap();
        let snapshot = build_package(&plan, &mut sink).unwrap();
        let authority = PackageAuthority {
            version: Version::parse(version).unwrap(),
            package_sha256: snapshot.package_sha256().to_owned(),
            inventory_sha256: snapshot.inventory_sha256().to_owned(),
            candidate_id: snapshot.candidate_id().to_owned(),
        };
        Bundle {
            plan,
            snapshot,
            authority,
        }
    }

    pub fn host(&self) -> HostCapabilityDeclaration {
        HostCapabilityDeclaration::isolated(
            &self.root,
            &self.project,
            "isolated-host-v1",
            Some(&std::env::current_exe().unwrap()),
        )
        .unwrap()
    }

    pub fn seed(&self, installed: Option<&Bundle>, cache: Option<&Bundle>) {
        for (path, bundle) in [
            ("installed/harness-ultragoal.hugpkg", installed),
            ("cache/harness-ultragoal.hugpkg", cache),
        ] {
            if let Some(bundle) = bundle {
                let file = ScopedFile::new(self.confined(), path).unwrap();
                assert!(file.apply(None, Some(bundle.snapshot.archive())).unwrap());
            }
        }
    }

    pub fn replace(&self, path: &str, expected: Option<&str>, bytes: Option<&[u8]>) {
        let file = ScopedFile::new(self.confined(), path).unwrap();
        assert!(file.apply(expected, bytes).unwrap());
    }

    pub fn session(
        &self,
        bundle: &Bundle,
        lifecycle: LifecyclePlan,
    ) -> (HostLifecycleSession, HostCapabilityDeclaration) {
        let host = self.host();
        let session = self.session_with_scope(
            bundle,
            lifecycle,
            host.clone(),
            HostScopeAuthority::Personal {
                marketplace: "local-harness-plugins".into(),
            },
        );
        (session, host)
    }

    pub fn session_with_scope(
        &self,
        bundle: &Bundle,
        lifecycle: LifecyclePlan,
        host: HostCapabilityDeclaration,
        host_scope: HostScopeAuthority,
    ) -> HostLifecycleSession {
        HostLifecycleSession::bind(
            self.confined(),
            &bundle.plan,
            &bundle.snapshot,
            lifecycle,
            host,
            marketplace_plan(bundle),
            host_scope,
        )
        .unwrap()
    }

    pub fn tree(&self) -> Vec<(String, &'static str, u32, Vec<u8>)> {
        tree(&self.root)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[derive(Clone)]
pub struct Reader {
    pub provenance_sha256: String,
    pub marketplace: Option<Vec<u8>>,
    pub cache: Option<Vec<u8>>,
    pub registry: Option<Vec<u8>>,
    pub ui: Option<Vec<u8>>,
    pub runtime: Option<RuntimeObservation>,
    pub mutate_marketplace_after_first: bool,
    pub marketplace_reads: usize,
    pub transaction_supported: bool,
    pub generation: u64,
}

impl Reader {
    pub fn empty(binding: &JourneyBinding) -> Self {
        Self {
            provenance_sha256: binding.binding_sha256().to_owned(),
            ..Self::default()
        }
    }

    pub fn complete(
        bundle: &Bundle,
        host: &HostCapabilityDeclaration,
        binding: &JourneyBinding,
    ) -> Self {
        Self {
            provenance_sha256: binding.binding_sha256().to_owned(),
            marketplace: Some(marketplace_plan(bundle).replacement().to_vec()),
            cache: Some(cache_document(bundle, host)),
            registry: Some(registry_document(binding, true, true).unwrap()),
            ui: None,
            runtime: Some(runtime_observation(binding, host)),
            mutate_marketplace_after_first: false,
            marketplace_reads: 0,
            transaction_supported: true,
            generation: 0,
        }
    }

    pub fn teardown(bundle: &Bundle, binding: &JourneyBinding) -> Self {
        Self {
            provenance_sha256: binding.binding_sha256().to_owned(),
            marketplace: Some(marketplace_plan(bundle).replacement().to_vec()),
            ..Self::default()
        }
    }

    pub fn read_marketplace_raw(&mut self) -> Result<Option<Vec<u8>>, ()> {
        self.marketplace_reads += 1;
        let mut value = self.marketplace.clone();
        if self.mutate_marketplace_after_first && self.marketplace_reads > 1 {
            value = Some(b"changed".to_vec());
        }
        Ok(value)
    }

    pub fn read_cache_raw(&self) -> Result<Option<Vec<u8>>, ()> {
        Ok(self.cache.clone())
    }

    pub fn read_registry_raw(&self) -> Result<Option<Vec<u8>>, ()> {
        Ok(self.registry.clone())
    }

    pub fn read_plugins_ui_raw(&self) -> Result<Option<Vec<u8>>, ()> {
        Ok(self.ui.clone())
    }

    pub fn read_runtime_raw(&self) -> Result<Option<RuntimeObservation>, ()> {
        Ok(self.runtime.clone())
    }
}

impl Default for Reader {
    fn default() -> Self {
        Self {
            provenance_sha256:
                "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
            marketplace: None,
            cache: None,
            registry: None,
            ui: None,
            runtime: None,
            mutate_marketplace_after_first: false,
            marketplace_reads: 0,
            transaction_supported: true,
            generation: 0,
        }
    }
}

impl HostSurfaceReader for Reader {
    fn with_transaction<T>(
        &mut self,
        request: &HostObservationTransactionRequest,
        operation: impl FnOnce(
            Result<&mut dyn HostSurfaceTransaction, HostSurfaceTransactionError>,
        ) -> T,
    ) -> T {
        if !self.transaction_supported {
            return operation(Err(HostSurfaceTransactionError::Unsupported));
        }
        let start_generation = self.generation;
        let mut transaction = ReaderTransaction {
            reader: self,
            host_scope_sha256: request.host_scope_sha256().to_owned(),
            session_issuance_sha256: request.session_issuance_sha256().to_owned(),
            start_generation,
        };
        operation(Ok(&mut transaction))
    }
}

pub struct ReaderTransaction<'a> {
    reader: &'a mut Reader,
    host_scope_sha256: String,
    session_issuance_sha256: String,
    start_generation: u64,
}

impl HostSurfaceTransaction for ReaderTransaction<'_> {
    fn provenance_sha256(&self) -> &str {
        &self.reader.provenance_sha256
    }

    fn host_scope_sha256(&self) -> &str {
        &self.host_scope_sha256
    }

    fn session_issuance_sha256(&self) -> &str {
        &self.session_issuance_sha256
    }

    fn start_generation(&self) -> u64 {
        self.start_generation
    }

    fn current_generation(&self) -> Result<u64, ()> {
        Ok(self.reader.generation)
    }

    fn read_marketplace(&mut self, _: usize) -> Result<Option<Vec<u8>>, ()> {
        self.reader.read_marketplace_raw()
    }

    fn read_cache(&mut self, _: usize) -> Result<Option<Vec<u8>>, ()> {
        self.reader.read_cache_raw()
    }

    fn read_registry(&mut self, _: usize) -> Result<Option<Vec<u8>>, ()> {
        self.reader.read_registry_raw()
    }

    fn read_plugins_ui(&mut self, _: usize) -> Result<Option<Vec<u8>>, ()> {
        self.reader.read_plugins_ui_raw()
    }

    fn read_runtime(&mut self) -> Result<Option<RuntimeObservation>, ()> {
        self.reader.read_runtime_raw()
    }
}

pub fn marketplace_plan(bundle: &Bundle) -> MarketplacePlan {
    marketplace_plan_named(bundle, "local-harness-plugins")
}

pub fn marketplace_plan_named(bundle: &Bundle, marketplace: &str) -> MarketplacePlan {
    plan_codex_marketplace(
        None,
        None,
        marketplace,
        "Local Harness Plugins",
        CodexPlugin::harness_ultragoal(),
        bundle.snapshot.identity().clone(),
    )
    .unwrap()
}

pub fn cache_document(bundle: &Bundle, host: &HostCapabilityDeclaration) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "schema":"harness-ultragoal.codex-cache-observation.v1",
        "context_id":CONTEXT, "candidate_id":CANDIDATE,
        "cache_root_id":host.home_id(),
        "entries":[{
            "marketplace":"local-harness-plugins", "plugin_id":"harness-ultragoal",
            "version":bundle.plan.version(),
            "package_tree_sha256":bundle.snapshot.identity().tree_sha256()
        }]
    }))
    .unwrap()
}

pub fn ui_document(binding: &JourneyBinding) -> Vec<u8> {
    let package = binding.package();
    serde_json::to_vec(&json!({
        "schema":"harness-ultragoal.plugins-ui-observation.v1",
        "context_id":package.source().context_id(),
        "candidate_id":package.source().candidate_id(),
        "home_id":binding.home_id(), "project_id":binding.project_id(),
        "host_id":binding.host_id(), "capability_sha256":binding.capability_sha256(),
        "binding_sha256":binding.binding_sha256(),
        "entries":[{
            "plugin_id":package.source().plugin_id(), "version":package.source().version(),
            "package_sha256":package.archive_sha256(),
            "installed_tree_sha256":package.tree_sha256(), "visible":true
        }]
    }))
    .unwrap()
}

pub fn installed(authority: &PackageAuthority, generation: u64) -> LifecycleState {
    LifecycleState {
        installed: Some(authority.clone()),
        cache: Some(authority.clone()),
        generation,
        recovery_required: false,
    }
}

pub fn request(
    intent: LifecycleIntent,
    target: Option<PackageAuthority>,
    prior: Option<LifecycleState>,
    current: Option<&PackageAuthority>,
    write: bool,
    downgrade: bool,
) -> LifecycleRequest {
    LifecycleRequest {
        intent,
        target,
        prior_authority: prior,
        authorization: LifecycleAuthorization {
            allow_host_write: write,
            allow_downgrade: downgrade,
            expected_installed_sha256: current.map(|row| row.package_sha256.clone()),
        },
    }
}

pub fn lifecycle(state: &LifecycleState, request: LifecycleRequest) -> LifecyclePlan {
    plan(state, &request).unwrap()
}

pub fn handoff_external_effect(session: &mut HostLifecycleSession) -> Option<HostCommandPlan> {
    if matches!(
        session.plan().intent,
        LifecycleIntent::RepeatUse | LifecycleIntent::IdempotentReinstall
    ) {
        return None;
    }
    let request = session.take_external_effect_request().unwrap();
    let prepared = session.consume_external_effect_request(request).unwrap();
    Some(prepared.into_plan().unwrap())
}

fn runtime_observation(
    binding: &JourneyBinding,
    host: &HostCapabilityDeclaration,
) -> RuntimeObservation {
    let plan = RuntimeProbePlan::new(
        binding.clone(),
        host,
        &std::env::current_exe().unwrap(),
        vec![
            "--exact".into(),
            "support::runtime_probe_child".into(),
            "--nocapture".into(),
        ],
        Duration::from_secs(10),
    )
    .unwrap();
    execute_runtime_probe(&plan).unwrap()
}

#[test]
fn runtime_probe_child() {
    if std::env::var_os("HUL_BINDING_SHA256").is_none() {
        return;
    }
    let value = json!({
        "schema":"harness-ultragoal.runtime-probe.v1",
        "context_id":env("HUL_CONTEXT_ID"), "candidate_id":env("HUL_CANDIDATE_ID"),
        "plugin_id":env("HUL_PLUGIN_ID"), "version":env("HUL_VERSION"),
        "package_sha256":env("HUL_PACKAGE_SHA256"),
        "installed_tree_sha256":env("HUL_TREE_SHA256"),
        "home_id":env("HUL_HOME_ID"), "project_id":env("HUL_PROJECT_ID"),
        "host_id":env("HUL_HOST_ID"), "capability_sha256":env("HUL_CAPABILITY_SHA256"),
        "binding_sha256":env("HUL_BINDING_SHA256"),
        "executable_sha256":env("HUL_EXECUTABLE_SHA256"),
        "session_nonce":env("HUL_SESSION_NONCE")
    });
    println!("HUL_RUNTIME_OBSERVATION={value}");
}

fn env(name: &str) -> String {
    std::env::var(name).unwrap()
}

fn tree(root: &Path) -> Vec<(String, &'static str, u32, Vec<u8>)> {
    fn visit(root: &Path, path: &Path, rows: &mut Vec<(String, &'static str, u32, Vec<u8>)>) {
        let mut entries = fs::read_dir(path)
            .unwrap()
            .map(|row| row.unwrap().path())
            .collect::<Vec<_>>();
        entries.sort();
        for path in entries {
            let metadata = fs::symlink_metadata(&path).unwrap();
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            let mode = permission_mode(&metadata);
            if metadata.is_dir() {
                rows.push((relative, "directory", mode, Vec::new()));
                visit(root, &path, rows);
            } else if metadata.file_type().is_symlink() {
                rows.push((
                    relative,
                    "symlink",
                    mode,
                    fs::read_link(&path)
                        .unwrap()
                        .to_string_lossy()
                        .as_bytes()
                        .to_vec(),
                ));
            } else {
                rows.push((relative, "file", mode, fs::read(&path).unwrap()));
            }
        }
    }
    let mut rows = Vec::new();
    visit(root, root, &mut rows);
    rows
}

#[cfg(unix)]
fn permission_mode(metadata: &fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode()
}

#[cfg(not(unix))]
fn permission_mode(_metadata: &fs::Metadata) -> u32 {
    0
}
