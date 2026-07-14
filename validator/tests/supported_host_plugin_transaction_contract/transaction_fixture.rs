use crate::distribution::{ConfinedRoot, PackageSnapshot, ScopedFile, build_package, plan_package};
use crate::host_lifecycle::{
    DarwinHostSurface, DarwinHostTransactionAdapter, DarwinHostTransactionPlan,
};
use serde_json::json;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub const CONTEXT: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
pub const CANDIDATE: &str =
    "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
pub const FOREIGN_CANDIDATE: &str =
    "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
pub const MARKETPLACE: &str = "local-harness-plugins";
static NEXT: AtomicU64 = AtomicU64::new(0);
static NEXT_PACKAGE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub struct Bundle {
    pub snapshot: PackageSnapshot,
}

pub struct Fixture {
    pub root: PathBuf,
    pub adapter: DarwinHostTransactionAdapter,
}

impl Fixture {
    pub fn new(label: &str) -> Self {
        let root = PathBuf::from("/tmp").join(format!(
            "hul-distribution-supported-host-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let adapter = DarwinHostTransactionAdapter::open(ConfinedRoot::open(&root).unwrap())
            .expect("Darwin adapter");
        Self { root, adapter }
    }

    pub fn bundle(&self, version: &str) -> Bundle {
        self.bundle_with_candidate(version, CANDIDATE)
    }

    pub fn bundle_with_candidate(&self, version: &str, candidate: &str) -> Bundle {
        let token = format!(
            "{}-{}-{}",
            version.replace('.', "-"),
            &candidate[7..15],
            NEXT_PACKAGE.fetch_add(1, Ordering::Relaxed),
        );
        let source = self.root.join(format!("source-{token}"));
        fs::create_dir(&source).unwrap();
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
            "schema":"harness-ultragoal.package-plan.v1",
            "context_id":CONTEXT,
            "candidate_id":candidate,
            "plugin_id":"harness-ultragoal",
            "version":version,
            "source_date_epoch":1_700_000_000u64,
            "entries":[
                {"path":".codex-plugin/plugin.json","source_path":format!("source-{token}/plugin.json"),"role":"manifest","executable":false},
                {"path":"skills/harness-ultragoal/SKILL.md","source_path":format!("source-{token}/front.md"),"role":"skill","executable":false}
            ]
        });
        let plan = plan_package(&self.root, &serde_json::to_vec(&spec).unwrap()).unwrap();
        let mut sink = ScopedFile::new(
            ConfinedRoot::open(&self.root).unwrap(),
            &format!("packages/harness-ultragoal-{token}.hugpkg"),
        )
        .unwrap();
        let snapshot = build_package(&plan, &mut sink).unwrap();
        Bundle { snapshot }
    }

    pub fn install_plan(&self, bundle: &Bundle) -> DarwinHostTransactionPlan {
        self.adapter
            .plan_install(&bundle.snapshot, MARKETPLACE)
            .unwrap()
    }

    pub fn reinstall_plan(&self, bundle: &Bundle) -> DarwinHostTransactionPlan {
        self.adapter
            .plan_reinstall(&bundle.snapshot, MARKETPLACE)
            .unwrap()
    }

    pub fn record_path(&self, surface: DarwinHostSurface) -> PathBuf {
        self.root.join(surface.relative_path()).join("record")
    }

    pub fn journal_record_path(&self) -> PathBuf {
        self.root.join("host-lifecycle/transaction/record")
    }

    pub fn lineage_record_path(&self) -> PathBuf {
        self.root.join("host-lifecycle/lineage/record")
    }

    pub fn lineage_anchor_record_path(&self) -> PathBuf {
        self.root.join("host-lifecycle/lineage-anchor/record")
    }

    pub fn remove_surface(&self, surface: DarwinHostSurface) {
        fs::remove_dir_all(self.root.join(surface.relative_path())).unwrap();
    }

    pub fn overwrite_record(&self, surface: DarwinHostSurface, bytes: &[u8]) {
        fs::write(self.record_path(surface), bytes).unwrap();
    }

    pub fn chmod_record(&self, surface: DarwinHostSurface, mode: u32) {
        fs::set_permissions(self.record_path(surface), fs::Permissions::from_mode(mode)).unwrap();
    }

    pub fn tree(&self) -> Vec<TreeRow> {
        let mut rows = Vec::new();
        visit(&self.root, &self.root, &mut rows);
        rows.sort_by(|left, right| left.path.cmp(&right.path));
        rows
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreeRow {
    path: String,
    kind: &'static str,
    mode: u32,
    bytes: Vec<u8>,
}

fn visit(root: &Path, path: &Path, rows: &mut Vec<TreeRow>) {
    let mut entries = fs::read_dir(path)
        .unwrap()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    entries.sort_by_key(|row| row.file_name());
    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).unwrap();
        let relative = path
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let (kind, bytes) = if metadata.file_type().is_symlink() {
            (
                "symlink",
                fs::read_link(&path)
                    .unwrap()
                    .to_string_lossy()
                    .as_bytes()
                    .to_vec(),
            )
        } else if metadata.is_dir() {
            ("directory", Vec::new())
        } else if metadata.is_file() {
            ("file", fs::read(&path).unwrap())
        } else {
            ("special", Vec::new())
        };
        rows.push(TreeRow {
            path: relative,
            kind,
            mode: metadata.permissions().mode() & 0o777,
            bytes,
        });
        if metadata.is_dir() {
            visit(root, &path, rows);
        }
    }
}
