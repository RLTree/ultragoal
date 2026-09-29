use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::UNIX_EPOCH;
use ultragoal::context::BuildRequest;
use ultragoal::inventory::{ADOPTED_HANDOFF_DIGEST_CONFIG_KEY, ADOPTED_HANDOFF_MANIFEST_SHA256};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

static NEXT: AtomicU64 = AtomicU64::new(0);

pub fn inventory_request(start: &Path) -> BuildRequest {
    BuildRequest::new(start).bind_non_secret_configuration(
        ADOPTED_HANDOFF_DIGEST_CONFIG_KEY,
        ADOPTED_HANDOFF_MANIFEST_SHA256,
    )
}

pub fn live_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("validator has repository parent")
        .to_path_buf()
}

pub struct TestRepo {
    pub root: PathBuf,
}

impl TestRepo {
    pub fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ultragoal-inventory-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        run_git(&root, &["init", "-q"]);
        run_git(
            &root,
            &["config", "user.email", "inventory@example.invalid"],
        );
        run_git(&root, &["config", "user.name", "Inventory Test"]);
        let contract = root.join("docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT");
        fs::create_dir_all(&contract).unwrap();
        let source =
            live_root().join("docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT");
        let mut contract_sources = fs::read_dir(&source)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.is_file())
            .collect::<Vec<_>>();
        contract_sources.sort();
        for path in contract_sources {
            fs::copy(&path, contract.join(path.file_name().unwrap())).unwrap();
        }
        fs::copy(
            source
                .parent()
                .unwrap()
                .join("FINAL-HANDOFF-MANIFEST.sha256"),
            contract
                .parent()
                .unwrap()
                .join("FINAL-HANDOFF-MANIFEST.sha256"),
        )
        .unwrap();
        fs::copy(
            source.parent().unwrap().join("README.md"),
            contract.parent().unwrap().join("README.md"),
        )
        .unwrap();
        let routes = root.join("migration/authority-routes.json");
        fs::create_dir_all(routes.parent().unwrap()).unwrap();
        fs::copy(live_root().join("migration/authority-routes.json"), &routes).unwrap();
        fs::copy(
            live_root().join("migration/generated-surface-authority.json"),
            root.join("migration/generated-surface-authority.json"),
        )
        .unwrap();
        crate::generated_authority_fixture::copy_declared_files(&root);
        fs::copy(
            live_root().join("migration/non-authoritative-contexts.json"),
            root.join("migration/non-authoritative-contexts.json"),
        )
        .unwrap();
        fs::copy(
            live_root().join("LANE_REGISTRY.json"),
            root.join("LANE_REGISTRY.json"),
        )
        .unwrap();
        fs::create_dir_all(root.join("templates")).unwrap();
        fs::copy(
            live_root().join("templates/LANE_REGISTRY.json"),
            root.join("templates/LANE_REGISTRY.json"),
        )
        .unwrap();
        fs::create_dir_all(root.join(".codex-plugin")).unwrap();
        fs::write(
            root.join(".codex-plugin/plugin.json"),
            br#"{"name":"harness-ultragoal","version":"0.0.0-test","description":"fixture","author":{"name":"Inventory Test"},"skills":"./skills/","interface":{"displayName":"Harness Ultragoal","shortDescription":"fixture","longDescription":"fixture manifest","developerName":"Inventory Test","category":"Productivity","capabilities":["Read"],"defaultPrompt":["Inspect this fixture."]}}"#,
        )
        .unwrap();
        Self { root }
    }

    pub fn skill(&self, directory: &str, declared_name: &str) {
        let path = self.root.join("skills").join(directory);
        fs::create_dir_all(&path).unwrap();
        fs::write(
            path.join("SKILL.md"),
            format!("---\nname: {declared_name}\ndescription: fixture\n---\n"),
        )
        .unwrap();
    }

    pub fn write(&self, relative: &str, bytes: &[u8]) {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, bytes).unwrap();
    }

    pub fn commit(&self) {
        run_git(&self.root, &["add", "-A"]);
        run_git(&self.root, &["commit", "-q", "-m", "fixture"]);
    }
}

impl Drop for TestRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn run_git(root: &Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .args(args)
            .current_dir(root)
            .status()
            .unwrap()
            .success()
    );
}

#[derive(Debug, Eq, PartialEq)]
pub struct SnapshotRow {
    path: PathBuf,
    kind: &'static str,
    size: u64,
    digest: String,
    mode: Option<u32>,
    modified_ns: Option<u128>,
}

fn digest(path: &Path, metadata: &fs::Metadata) -> String {
    let bytes = if metadata.is_file() {
        fs::read(path).unwrap()
    } else if metadata.file_type().is_symlink() {
        fs::read_link(path)
            .unwrap()
            .to_string_lossy()
            .as_bytes()
            .to_vec()
    } else {
        Vec::new()
    };
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn walk(root: &Path, current: &Path, rows: &mut Vec<SnapshotRow>) {
    let mut paths = fs::read_dir(current)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    paths.sort();
    for path in paths {
        let metadata = fs::symlink_metadata(&path).unwrap();
        #[cfg(unix)]
        let mode = Some(metadata.permissions().mode());
        #[cfg(not(unix))]
        let mode = None;
        rows.push(SnapshotRow {
            path: path.strip_prefix(root).unwrap().to_path_buf(),
            kind: if metadata.is_file() {
                "file"
            } else if metadata.is_dir() {
                "directory"
            } else if metadata.file_type().is_symlink() {
                "symlink"
            } else {
                "special"
            },
            size: metadata.len(),
            digest: digest(&path, &metadata),
            mode,
            modified_ns: metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_nanos()),
        });
        if metadata.is_dir() {
            walk(root, &path, rows);
        }
    }
}

pub fn snapshot(root: &Path) -> Vec<SnapshotRow> {
    let mut rows = Vec::new();
    walk(root, root, &mut rows);
    rows
}
