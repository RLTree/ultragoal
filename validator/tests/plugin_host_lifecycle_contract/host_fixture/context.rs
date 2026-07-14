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
        HostLifecycleSession::bind(HostLifecycleBindRequest {
            root: self.confined(),
            package_plan: &bundle.plan,
            package: &bundle.snapshot,
            lifecycle,
            host,
            marketplace_plan: marketplace_plan(bundle),
            host_scope,
        })
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
