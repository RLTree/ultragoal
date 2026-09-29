use super::digest;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

pub(super) struct Fixture {
    parent: PathBuf,
    pub(super) root: PathBuf,
    worktree_root: PathBuf,
    pub(super) fit: PathBuf,
    routine: PathBuf,
    pub(super) commit: String,
    tree: String,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let parent = std::env::temp_dir().join(format!(
            "ultragoal-p0-worktrees-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        let root = parent.join("root");
        let worktree_root = parent.join("worktrees");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&worktree_root).unwrap();
        git(&root, &["init", "-q"]);
        git(&root, &["config", "user.email", "p0@example.invalid"]);
        git(&root, &["config", "user.name", "P0 Test"]);
        fs::write(root.join("seed"), b"seed").unwrap();
        git(&root, &["add", "seed"]);
        git(&root, &["commit", "-q", "-m", "base"]);
        let commit = output(&root, &["rev-parse", "HEAD"]);
        let tree = output(&root, &["rev-parse", "HEAD^{tree}"]);
        let fit = worktree_root.join("fit");
        let routine = worktree_root.join("routine");
        add_worktree(&root, "codex/p0-fit", &fit);
        add_worktree(&root, "codex/p0-routine", &routine);
        Self {
            parent,
            root: fs::canonicalize(root).unwrap(),
            worktree_root: fs::canonicalize(worktree_root).unwrap(),
            fit: fs::canonicalize(fit).unwrap(),
            routine: fs::canonicalize(routine).unwrap(),
            commit,
            tree,
        }
    }

    pub(super) fn base(&self) -> (&str, &str) {
        (&self.commit, &self.tree)
    }

    pub(super) fn records(&self) -> Vec<Value> {
        vec![
            self.record("fit", "src/fit.rs", "src/fit_support.rs", &self.fit),
            self.record(
                "routine",
                "src/routine.rs",
                "src/routine_support.rs",
                &self.routine,
            ),
        ]
    }

    fn record(&self, name: &str, diagnostic: &str, dependency: &str, worktree: &Path) -> Value {
        json!({
            "lease_id": format!("P0-{name}"), "lane_id": "P0",
            "exception_id": "P0-DEBT-REPAIR", "base_commit": self.commit,
            "base_tree": self.tree, "branch": format!("codex/p0-{name}"),
            "worktree": worktree, "status": "issued", "diagnostic_paths": [diagnostic],
            "support_files": [dependency], "owned_files": [diagnostic, dependency],
            "diagnostic_source_kind": "clippy_diagnostics",
            "diagnostic_path_set_digest": path_digest(diagnostic),
            "operation_id": format!("p0-clippy-{}-{}", self.commit, self.tree),
            "tool": "clippy", "observed_at": "2026-07-19T00:00:00Z"
        })
    }

    pub(super) fn registry(&self) -> Value {
        let complete = BTreeSet::from(["src/fit.rs".to_owned(), "src/routine.rs".to_owned()]);
        let observation = |id: &str, command: &str, kind: &str, paths: &BTreeSet<String>| {
            json!({
                "command": command, "status": "current", "source_kind": kind, "output_path": null,
                "candidate_commit": self.commit, "candidate_tree": self.tree, "exit_code": 0,
                "operation_id": format!("p0-{id}-{}-{}", self.commit, self.tree),
                "tool": id, "observed_at": "2026-07-19T00:00:00Z",
                "diagnostic_path_set_digest": digest(paths)
            })
        };
        json!({
            "pre_adoption_source": {"eligible_scheduler_nodes": []},
            "lanes": [{"id": "N14", "state": "blocked"}],
            "lease_state": {
                "worktree_root": self.worktree_root,
                "p0_exception": {
                    "allowed": {"paths": ["src/fit.rs", "src/fit_support.rs", "src/routine.rs", "src/routine_support.rs"]},
                    "forbidden": {"paths": ["schemas/**"]},
                    "debt_path_sources": {
                        "compile": observation("compile", "cargo check --manifest-path validator/Cargo.toml --lib --message-format=short", "compile_diagnostics", &BTreeSet::new()),
                        "clippy": observation("clippy", "cargo clippy --locked --manifest-path validator/Cargo.toml --lib -- -D warnings", "clippy_diagnostics", &complete),
                        "namespace": observation("namespace", "target/debug/ultragoal --json --root . check strict --claim namespace-progressive-disclosure", "namespace_findings", &BTreeSet::new()),
                        "standards": observation("standards", "scripts/check-agent-standards .", "standards_findings", &BTreeSet::new()),
                        "retention": observation("retention", "authorized retention reconciliation of validation_artifacts/review and .codex-worktree home/tmp", "retention_inventory", &BTreeSet::new())
                    },
                    "issuance_transition": {
                        "status": "issued",
                        "source_sets": {"compile": [], "clippy": ["src/fit.rs", "src/routine.rs"], "namespace": [], "standards": [], "retention": []}
                    }
                }
            }
        })
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.parent);
    }
}

fn add_worktree(root: &Path, branch: &str, worktree: &Path) {
    git(
        root,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            branch,
            worktree.to_str().unwrap(),
            "HEAD",
        ],
    );
}

pub(super) fn git(root: &Path, arguments: &[&str]) {
    assert!(
        Command::new("git")
            .args(arguments)
            .current_dir(root)
            .status()
            .unwrap()
            .success()
    );
}

fn output(root: &Path, arguments: &[&str]) -> String {
    String::from_utf8(
        Command::new("git")
            .args(arguments)
            .current_dir(root)
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_owned()
}

pub(super) fn path_digest(path: &str) -> String {
    digest(&BTreeSet::from([path.to_owned()]))
}

pub(super) fn empty_digest() -> String {
    digest(&BTreeSet::new())
}
