use crate::context::{BuildRequest, LiveContext};
use crate::repository_fit::local::LocalEffects;
use crate::repository_fit::product_adapter::catalog::CANONICAL_TEMPLATES;
use crate::repository_fit::product_adapter::{
    PreparedFitApply, plan_target, prepare_apply_request,
};
use crate::repository_fit::{FitError, FitVerification, digest};
use std::fs;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);
const BASE: &str = "/tmp/hul-repository-fit-public-adapter-056";

pub(super) struct Fixture {
    pub(super) container: PathBuf,
    pub(super) root: PathBuf,
}

impl Fixture {
    pub(super) fn new(label: &str) -> Self {
        let nonce = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let container = PathBuf::from(BASE).join(format!(
            "{}-{}-{nonce}",
            label.replace(|character: char| !character.is_ascii_alphanumeric(), "-"),
            std::process::id()
        ));
        let root = container.join("repo");
        fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "--quiet"]);
        Self { container, root }
    }

    pub(super) fn context(&self) -> LiveContext {
        LiveContext::build(BuildRequest::new(&self.root)).unwrap()
    }

    pub(super) fn write(&self, relative: &str, bytes: &[u8]) {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    pub(super) fn write_template(&self, target: &str) {
        let row = CANONICAL_TEMPLATES
            .iter()
            .find(|row| row.target_path == target)
            .unwrap();
        self.write(row.target_path, row.bytes);
        fs::set_permissions(
            self.root.join(row.target_path),
            fs::Permissions::from_mode(row.unix_mode),
        )
        .unwrap();
    }

    pub(super) fn install_all_direct(&self) {
        for row in CANONICAL_TEMPLATES {
            self.write(row.target_path, row.bytes);
            fs::set_permissions(
                self.root.join(row.target_path),
                fs::Permissions::from_mode(row.unix_mode),
            )
            .unwrap();
        }
    }

    pub(super) fn plan(&self, context: &LiveContext) -> PreparedFitApply {
        let record = plan_target(context).unwrap();
        let plan_sha256 = record.plan_sha256().to_owned();
        let bytes = record.to_machine_bytes().unwrap();
        prepare_apply_request(context, &bytes, &plan_sha256).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        debug_assert!(self.container.starts_with(BASE));
        let _ = fs::remove_dir_all(&self.container);
    }
}

pub(super) fn effects_for(fixture: &Fixture, prepared: &PreparedFitApply) -> LocalEffects {
    LocalEffects::open_for_test(&fixture.root, prepared.request().unix_modes().clone()).unwrap()
}

pub(super) fn execute(
    fixture: &Fixture,
    prepared: PreparedFitApply,
) -> Result<FitVerification, FitError> {
    let mut effects = effects_for(fixture, &prepared);
    prepared.into_request().execute_for_test(&mut effects)
}

pub(super) fn git_status(root: &Path) -> Vec<u8> {
    let output = Command::new("git")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args([
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.untrackedCache=false",
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
        ])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output.stderr);
    output.stdout
}

pub(super) fn snapshot(root: &Path) -> Vec<SnapshotRow> {
    fn visit(root: &Path, path: &Path, rows: &mut Vec<SnapshotRow>) {
        let mut entries = fs::read_dir(path)
            .unwrap()
            .map(Result::unwrap)
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            let metadata = fs::symlink_metadata(&path).unwrap();
            let (kind, payload) = if metadata.file_type().is_symlink() {
                (
                    "symlink",
                    fs::read_link(&path).unwrap().to_string_lossy().into_owned(),
                )
            } else if metadata.is_dir() {
                visit(root, &path, rows);
                ("directory", String::new())
            } else if metadata.is_file() {
                ("file", digest(&fs::read(&path).unwrap()))
            } else if metadata.file_type().is_fifo() {
                ("fifo", String::new())
            } else if metadata.file_type().is_socket() {
                ("socket", String::new())
            } else {
                ("special", String::new())
            };
            rows.push(SnapshotRow {
                relative,
                kind: kind.to_owned(),
                payload,
                mode: metadata.mode(),
                links: metadata.nlink(),
            });
        }
    }
    let mut rows = Vec::new();
    visit(root, root, &mut rows);
    rows.sort();
    rows
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct SnapshotRow {
    relative: String,
    kind: String,
    payload: String,
    mode: u32,
    links: u64,
}

pub(super) fn assert_zero_write<T>(fixture: &Fixture, operation: impl FnOnce() -> T) -> T {
    let status = git_status(&fixture.root);
    let before = snapshot(&fixture.root);
    let result = operation();
    assert_eq!(git_status(&fixture.root), status);
    assert_eq!(snapshot(&fixture.root), before);
    result
}

fn git(root: &Path, arguments: &[&str]) {
    let status = Command::new("git")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args(arguments)
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success());
}
