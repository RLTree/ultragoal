use super::super::context::{BuildRequest, LiveContext};
use super::successor::{EffectClass, ParseOutcome, parse_args};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use ultragoal::inventory::{ADOPTED_HANDOFF_DIGEST_CONFIG_KEY, ADOPTED_HANDOFF_MANIFEST_SHA256};

static NEXT_REPOSITORY: AtomicU64 = AtomicU64::new(0);

#[path = "state_views.rs"]
mod state_views;

struct Repository {
    root: PathBuf,
}

impl Repository {
    fn new(label: &str) -> Self {
        let sequence = NEXT_REPOSITORY.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "ultragoal-successor-runtime-{label}-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create runtime repository");
        let repository = Self { root };
        repository.git(&["init", "-q"]);
        repository.git(&["config", "user.email", "runtime@example.invalid"]);
        repository.git(&["config", "user.name", "Runtime Test"]);
        fs::write(repository.root.join("tracked.txt"), b"tracked\n")
            .expect("write tracked fixture");
        repository.git(&["add", "tracked.txt"]);
        repository.git(&["commit", "-q", "-m", "fixture"]);
        repository
    }

    fn git(&self, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(&self.root)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .status()
            .expect("run Git fixture command");
        assert!(status.success(), "git {args:?}");
    }

    fn status(&self) -> Vec<u8> {
        let output = Command::new("git")
            .args(["status", "--porcelain=v1", "-z", "--untracked-files=all"])
            .current_dir(&self.root)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .output()
            .expect("read Git status");
        assert!(output.status.success(), "git status failed");
        output.stdout
    }
}

impl Drop for Repository {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn parsed(args: &[&str]) -> super::successor::ParsedInvocation {
    let ParseOutcome::Invocation(invocation) =
        parse_args(args.iter().copied()).expect("parse successor invocation")
    else {
        panic!("expected invocation");
    };
    invocation
}

fn context(root: &Path) -> LiveContext {
    LiveContext::build(
        BuildRequest::new(root)
            .with_effect(EffectClass::Read)
            .bind_non_secret_configuration(
                ADOPTED_HANDOFF_DIGEST_CONFIG_KEY,
                ADOPTED_HANDOFF_MANIFEST_SHA256,
            ),
    )
    .expect("build production-equivalent read context")
}

fn tree_snapshot(root: &Path) -> Vec<(String, Vec<u8>)> {
    fn collect(root: &Path, path: &Path, rows: &mut Vec<(String, Vec<u8>)>) {
        let mut entries = fs::read_dir(path)
            .expect("read snapshot directory")
            .map(|entry| entry.expect("read snapshot entry"))
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .expect("snapshot path under root")
                .to_string_lossy()
                .into_owned();
            let metadata = fs::symlink_metadata(&path).expect("read snapshot metadata");
            if metadata.is_dir() {
                rows.push((format!("directory:{relative}"), Vec::new()));
                collect(root, &path, rows);
            } else if metadata.file_type().is_symlink() {
                rows.push((
                    format!("symlink:{relative}"),
                    fs::read_link(&path)
                        .expect("read snapshot symlink")
                        .to_string_lossy()
                        .as_bytes()
                        .to_vec(),
                ));
            } else {
                rows.push((
                    format!("file:{relative}"),
                    fs::read(&path).expect("read snapshot file"),
                ));
            }
        }
    }

    let mut rows = Vec::new();
    collect(root, root, &mut rows);
    rows
}
