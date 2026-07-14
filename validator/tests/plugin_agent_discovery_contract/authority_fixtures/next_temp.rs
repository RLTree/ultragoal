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
