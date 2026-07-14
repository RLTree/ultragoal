use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use ultragoal::distribution::{
    ConfinedRoot, PackagePlan, PackageSnapshot, ScopedFile, build_package, plan_package,
};
use ultragoal::plugin_product::distribution_adapter::DistributionLifecycleOperation;
use ultragoal::plugin_product::lifecycle::{
    LifecycleAuthorization, LifecycleIntent, LifecyclePlan, LifecycleRequest, LifecycleState,
    PackageAuthority, Version, plan,
};

pub const CONTEXT: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
pub const CANDIDATE: &str =
    "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
pub const OTHER_CANDIDATE: &str =
    "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
static NEXT: AtomicU64 = AtomicU64::new(0);

pub struct Bundle {
    pub plan: PackagePlan,
    pub snapshot: PackageSnapshot,
    pub authority: PackageAuthority,
}

pub struct Fixture {
    pub root: PathBuf,
}

impl Fixture {
    pub fn new(_label: &str) -> Self {
        let temporary = std::env::var_os("CODEX_WORKTREE_TMP")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/tmp"));
        fs::create_dir_all(&temporary).unwrap();
        let root = temporary.join(format!(
            "hul-distribution-pa-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self { root }
    }

    pub fn confined(&self) -> ConfinedRoot {
        ConfinedRoot::open(&self.root).unwrap()
    }

    pub fn bundle(&self, version: &str) -> Bundle {
        let label = version.replace('.', "-");
        let source = self.root.join(format!("source-{label}"));
        fs::create_dir_all(&source).unwrap();
        let manifest = json!({
            "name":"harness-ultragoal",
            "version":version,
            "description":"Repository fit, routine work, diagnosis, proof, and migration.",
            "author":{"name":"Terry Noblin","email":"tree@terrynoblin.dev","url":"https://terrynoblin.dev"},
            "homepage":"https://terrynoblin.dev/harness-ultragoal",
            "repository":"https://github.com/terrynoblin/harness-ultragoal",
            "license":"UNLICENSED",
            "keywords":["agent-first","verification"],
            "skills":"./skills/",
            "interface":{
                "displayName":"Harness Ultragoal",
                "shortDescription":"One evidence-bound front door.",
                "longDescription":"Route repository work through explicit authority and effects.",
                "developerName":"Terry Noblin",
                "category":"Productivity",
                "capabilities":["Read","Write"],
                "websiteURL":"https://terrynoblin.dev/harness-ultragoal",
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
            "schema":"harness-ultragoal.package-plan.v1",
            "context_id":CONTEXT,
            "candidate_id":CANDIDATE,
            "plugin_id":"harness-ultragoal",
            "version":version,
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

    pub fn operation(
        &self,
        bundle: &Bundle,
        lifecycle: &LifecyclePlan,
    ) -> DistributionLifecycleOperation {
        DistributionLifecycleOperation::bind(
            self.confined(),
            &bundle.plan,
            &bundle.snapshot,
            lifecycle,
        )
        .unwrap()
    }

    pub fn replace(&self, path: &str, expected: Option<&str>, bytes: Option<&[u8]>) {
        let file = ScopedFile::new(self.confined(), path).unwrap();
        assert!(file.apply(expected, bytes).unwrap());
    }

    pub fn tree(&self) -> Vec<(String, &'static str, Vec<u8>)> {
        tree(&self.root)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
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

fn tree(root: &Path) -> Vec<(String, &'static str, Vec<u8>)> {
    fn visit(root: &Path, path: &Path, rows: &mut Vec<(String, &'static str, Vec<u8>)>) {
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
            if metadata.is_dir() {
                rows.push((relative, "directory", Vec::new()));
                visit(root, &path, rows);
            } else if metadata.file_type().is_symlink() {
                rows.push((
                    relative,
                    "symlink",
                    fs::read_link(&path)
                        .unwrap()
                        .to_string_lossy()
                        .as_bytes()
                        .to_vec(),
                ));
            } else {
                rows.push((relative, "file", fs::read(&path).unwrap()));
            }
        }
    }
    let mut rows = Vec::new();
    visit(root, root, &mut rows);
    rows
}
