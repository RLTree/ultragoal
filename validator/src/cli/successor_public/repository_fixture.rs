use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

pub(super) struct Repository {
    pub(super) root: PathBuf,
}

impl Repository {
    pub(super) fn new(label: &str) -> Self {
        let live = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let root = std::env::temp_dir().join(format!(
            "ultragoal-successor-public-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        let root = fs::canonicalize(root).unwrap();
        git(&root, &["init", "-q"]);
        git(&root, &["config", "user.email", "public@example.invalid"]);
        git(&root, &["config", "user.name", "Successor Public"]);
        copy_authority_inputs(&live, &root);
        git(&root, &["add", "-A"]);
        git(&root, &["commit", "-qm", "fixture"]);
        Self { root }
    }

    pub(super) fn status(&self) -> Vec<u8> {
        let output = Command::new("git")
            .args(["status", "--porcelain=v1", "-z", "--untracked-files=all"])
            .env("GIT_OPTIONAL_LOCKS", "0")
            .current_dir(&self.root)
            .output()
            .unwrap();
        assert!(output.status.success());
        output.stdout
    }

    pub(super) fn install_agent_authority(&self, home: &Path) {
        let version = serde_json::from_slice::<serde_json::Value>(
            &fs::read(self.root.join(".codex-plugin/plugin.json")).unwrap(),
        )
        .unwrap()["version"]
            .as_str()
            .unwrap()
            .to_owned();
        let installed = home.join(".codex/plugins/harness-ultragoal");
        let cache = home.join(format!(
            ".codex/plugins/cache/local-harness-plugins/harness-ultragoal/{version}"
        ));
        for target in [&installed, &cache] {
            copy_plugin_authority(&self.root, target);
        }
        fs::create_dir_all(home.join(".codex/agents")).unwrap();
    }
}

impl Drop for Repository {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn copy_authority_inputs(live: &Path, root: &Path) {
    let source = live.join("docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT");
    let target = root.join("docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT");
    fs::create_dir_all(&target).unwrap();
    let mut files = fs::read_dir(&source)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    files.sort();
    for path in files {
        fs::copy(&path, target.join(path.file_name().unwrap())).unwrap();
    }
    for name in ["FINAL-HANDOFF-MANIFEST.sha256", "README.md"] {
        fs::copy(
            source.parent().unwrap().join(name),
            target.parent().unwrap().join(name),
        )
        .unwrap();
    }
    fs::create_dir_all(root.join("migration")).unwrap();
    for name in ["authority-routes.json", "generated-surface-authority.json"] {
        fs::copy(
            live.join("migration").join(name),
            root.join("migration").join(name),
        )
        .unwrap();
    }
    fs::create_dir_all(root.join(".codex-plugin")).unwrap();
    fs::copy(
        live.join(".codex-plugin/plugin.json"),
        root.join(".codex-plugin/plugin.json"),
    )
    .unwrap();
    fs::write(root.join(".gitignore"), b"validation_artifacts/\n").unwrap();
    let agents = root.join(".codex/agents");
    fs::create_dir_all(&agents).unwrap();
    for entry in fs::read_dir(live.join(".codex/agents")).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), agents.join(entry.file_name())).unwrap();
    }
}

fn copy_plugin_authority(source: &Path, target: &Path) {
    let agents = target.join(".codex/agents");
    let plugin = target.join(".codex-plugin");
    fs::create_dir_all(&agents).unwrap();
    fs::create_dir_all(&plugin).unwrap();
    for entry in fs::read_dir(source.join(".codex/agents")).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), agents.join(entry.file_name())).unwrap();
    }
    fs::copy(
        source.join(".codex-plugin/plugin.json"),
        plugin.join("plugin.json"),
    )
    .unwrap();
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}: {output:?}");
}

pub(super) fn tree(root: &Path) -> Vec<(String, Vec<u8>)> {
    fn collect(root: &Path, current: &Path, rows: &mut Vec<(String, Vec<u8>)>) {
        let mut entries = fs::read_dir(current)
            .unwrap()
            .map(|entry| entry.unwrap())
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let relative = path.strip_prefix(root).unwrap().to_string_lossy();
            let metadata = fs::symlink_metadata(&path).unwrap();
            if metadata.is_dir() {
                rows.push((format!("directory:{relative}"), Vec::new()));
                collect(root, &path, rows);
            } else if metadata.file_type().is_symlink() {
                rows.push((
                    format!("symlink:{relative}"),
                    fs::read_link(&path)
                        .unwrap()
                        .to_string_lossy()
                        .as_bytes()
                        .to_vec(),
                ));
            } else {
                rows.push((format!("file:{relative}"), fs::read(&path).unwrap()));
            }
        }
    }
    let mut rows = Vec::new();
    collect(root, root, &mut rows);
    rows
}
