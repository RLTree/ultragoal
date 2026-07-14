use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) struct SourceRoot {
    path: PathBuf,
}

#[derive(Debug, Eq, PartialEq)]
pub(super) struct SnapshotEntry {
    relative: String,
    bytes: Vec<u8>,
    modified_nanos: u64,
    mode: PlatformFileMode,
}

#[derive(Debug, Eq, PartialEq)]
enum PlatformFileMode {
    #[cfg(unix)]
    Unix(u32),
    #[cfg(not(unix))]
    Unavailable,
}

impl SourceRoot {
    pub(super) fn new(label: &str) -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::current_dir()
            .expect("current directory")
            .join(".git/codex-test")
            .join(format!("self-law-{label}-{stamp}"));
        let root = Self { path };
        root.seed();
        root
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn write(&self, relative: &str, text: &str) {
        let path = self.path.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
        fs::write(path, text).expect("write fixture source");
    }

    pub(super) fn executable(&self, relative: &str, text: &str) {
        self.write(relative, text);
        #[cfg(unix)]
        {
            let path = self.path.join(relative);
            let mut permissions = fs::metadata(&path).expect("metadata").permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(path, permissions).expect("executable permission");
        }
    }

    fn seed(&self) {
        self.write("Cargo.toml", "[workspace]\nmembers = [\"validator\"]\n");
        self.write("Cargo.lock", "# fixture lock\n");
        self.write(
            "rust-toolchain.toml",
            "[toolchain]\nchannel = \"stable\"\nprofile = \"minimal\"\ncomponents = [\"rustfmt\"]\n",
        );
        self.write("plugin-manifest-draft.json", "{\"name\":\"fixture\"}\n");
        self.write("package.json", "{\"packageManager\":\"pnpm@10.13.1\"}\n");
        self.write("pnpm-workspace.yaml", "packages: []\n");
        self.write("pnpm-lock.yaml", "lockfileVersion: '9.0'\n");
        self.write(
            "validator/Cargo.toml",
            "[package]\nname = \"fixture\"\nversion = \"0.0.0\"\n",
        );
        self.write("validator/build.rs", "fn main() {}\n");
        self.write("validator/src/lib.rs", "pub fn product() {}\n");
        self.write(
            "validator/tests/product_contract.rs",
            "#[test] fn product() {}\n",
        );
        self.write("validator/build_support/template.rs", "pub fn stage() {}\n");
        self.write("validator/examples/inventory.rs", "fn main() {}\n");
        for name in [
            "AGENTS.md",
            "AGENT_STANDARDS.md",
            "README.md",
            "ARCHITECTURE.md",
            "PLANS.md",
            "SECURITY.md",
            "DESIGN.md",
            "FRONTEND.md",
            "RELIABILITY.md",
            "PRODUCT_SENSE.md",
            "PRODUCT_FITNESS.md",
            "QUALITY_SCORE.md",
            "PRODUCT_SUCCESS_CONTRACT.md",
        ] {
            self.write(name, "# Live standard\n");
        }
        self.executable("scripts/check", "#!/bin/sh\nexit 0\n");
        self.executable("scripts/project-generated-authority", "#!/bin/sh\nexit 0\n");
        self.write(".harness/coverage-command", "scripts/check\n");
        self.executable("templates/scripts/check", "#!/bin/sh\nexit 0\n");
        self.write("templates/agent-standards/policy.md", "# Policy\n");
        self.write("agent-standards/README.md", "# Standards\n");
        self.write("schemas/product.schema.json", "{\"type\":\"object\"}\n");
        self.write("docs/product-law.md", "# Product law\n");
        super::standards_fixture::seed(self);
        self.write("skills/product/SKILL.md", "# Product skill\n");
        self.write("agents/product-reviewer.md", "# Product reviewer\n");
        self.write(".codex-plugin/plugin.json", "{\"name\":\"fixture\"}\n");
    }
}

impl Drop for SourceRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub(super) fn lines(count: usize) -> String {
    (0..count).map(|_| "// governed\n").collect()
}

pub(super) fn snapshot(root: &Path) -> Vec<SnapshotEntry> {
    let mut rows = Vec::new();
    walk(root, root, &mut rows);
    rows.sort_by(|left, right| left.relative.cmp(&right.relative));
    rows
}

fn walk(root: &Path, directory: &Path, rows: &mut Vec<SnapshotEntry>) {
    let mut entries = fs::read_dir(directory)
        .expect("read snapshot directory")
        .map(|entry| entry.expect("entry").path())
        .collect::<Vec<_>>();
    entries.sort();
    for path in entries {
        let metadata = fs::symlink_metadata(&path).expect("snapshot metadata");
        if metadata.is_dir() {
            walk(root, &path, rows);
        } else if metadata.is_file() {
            rows.push(SnapshotEntry {
                relative: path
                    .strip_prefix(root)
                    .expect("relative")
                    .to_string_lossy()
                    .into_owned(),
                bytes: fs::read(&path).expect("snapshot bytes"),
                modified_nanos: modified(&metadata),
                mode: platform_file_mode(&metadata),
            });
        }
    }
}

#[cfg(unix)]
fn platform_file_mode(metadata: &fs::Metadata) -> PlatformFileMode {
    PlatformFileMode::Unix(metadata.permissions().mode())
}

#[cfg(not(unix))]
fn platform_file_mode(_: &fs::Metadata) -> PlatformFileMode {
    PlatformFileMode::Unavailable
}

fn modified(metadata: &fs::Metadata) -> u64 {
    metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_nanos() as u64)
        .unwrap_or(0)
}
