use super::lane_binding::DependencyIdentity;
use crate::context::LiveContext;
use crate::state::StateError;
use std::path::Path;
use std::process::{Command, ExitStatus};

const OWNED_SOURCE_PATHS: [&str; 10] = [
    "validator/src/cli/successor_public/strict/claim_reconciliation_stage_adapter.rs",
    "validator/src/cli/successor_public/strict/failure_diagnostics.rs",
    "validator/src/cli/successor_public/strict/mod.rs",
    "validator/src/cli/successor_public/strict/tests.rs",
    "validator/src/state/adopted.rs",
    "validator/src/state/adopted_claims",
    "validator/src/state/adopted_registry.rs",
    "validator/src/state/mod.rs",
    "validator/src/state/tests/adopted",
    "validator/src/state/tests/mod.rs",
];

pub(super) fn verify(context: &LiveContext, source: &DependencyIdentity) -> Result<(), StateError> {
    let candidate = context.candidate();
    let head = candidate
        .head_commit
        .as_deref()
        .filter(|_| !candidate.dirty)
        .ok_or_else(|| invalid("root-claim-source-candidate-dirty"))?;
    verify_source(context.worktree_root(), source, head)
}

fn verify_source(root: &Path, source: &DependencyIdentity, head: &str) -> Result<(), StateError> {
    let source_tree = git_text(root, &["rev-parse", &format!("{}^{{tree}}", source.commit)])?;
    if source_tree != source.tree
        || !git_status(root, &["merge-base", "--is-ancestor", &source.commit, head])?.success()
    {
        return Err(invalid("root-claim-source-identity-invalid"));
    }
    let mut args = vec![
        "diff",
        "--quiet",
        "--no-renames",
        &source.commit,
        head,
        "--",
    ];
    args.extend(OWNED_SOURCE_PATHS);
    if !git_status(root, &args)?.success() {
        return Err(invalid("root-claim-source-bytes-changed"));
    }
    Ok(())
}

fn git_text(root: &Path, args: &[&str]) -> Result<String, StateError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|_| invalid("root-claim-source-git-unavailable"))?;
    if !output.status.success() {
        return Err(invalid("root-claim-source-identity-unavailable"));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|_| invalid("root-claim-source-identity-not-utf8"))
}

fn git_status(root: &Path, args: &[&str]) -> Result<ExitStatus, StateError> {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .status()
        .map_err(|_| invalid("root-claim-source-git-unavailable"))
}

fn invalid(code: &str) -> StateError {
    StateError::InvalidCatalog(code.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn descendant_metadata_is_allowed_but_owned_source_changes_fail_closed() {
        let root = std::env::temp_dir().join(format!(
            "ultragoal-claim-source-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap();
        git(&root, &["init", "-q"]);
        git(&root, &["config", "user.email", "test@example.invalid"]);
        git(&root, &["config", "user.name", "Test"]);
        write(&root, OWNED_SOURCE_PATHS[0], "accepted\n");
        commit(&root, "accepted source");
        let source = identity(&root);

        write(&root, "docs/checkpoint.md", "registry checkpoint\n");
        commit(&root, "metadata descendant");
        let metadata_head = git_output(&root, &["rev-parse", "HEAD"]);
        assert!(verify_source(&root, &source, &metadata_head).is_ok());

        write(&root, OWNED_SOURCE_PATHS[0], "changed authority\n");
        commit(&root, "changed authority");
        let changed_head = git_output(&root, &["rev-parse", "HEAD"]);
        assert!(verify_source(&root, &source, &changed_head).is_err());

        let mut wrong_tree = source;
        wrong_tree.tree = "0".repeat(40);
        assert!(verify_source(&root, &wrong_tree, &metadata_head).is_err());
        fs::remove_dir_all(&root).unwrap();
    }

    fn identity(root: &Path) -> DependencyIdentity {
        DependencyIdentity {
            lane_id: "N12".to_owned(),
            commit: git_output(root, &["rev-parse", "HEAD"]),
            tree: git_output(root, &["rev-parse", "HEAD^{tree}"]),
        }
    }

    fn write(root: &Path, path: &str, bytes: &str) {
        let target = root.join(path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, bytes).unwrap();
    }

    fn commit(root: &Path, message: &str) {
        git(root, &["add", "."]);
        git(root, &["commit", "-q", "-m", message]);
    }

    fn git(root: &Path, args: &[&str]) {
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(root)
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    }

    fn git_output(root: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }
}
