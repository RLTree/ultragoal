use super::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

#[test]
fn descendant_metadata_is_allowed_but_owned_source_changes_fail_closed() {
    let root = test_root("identity");
    git_run(&root, &["init", "-q"]);
    git_run(&root, &["config", "user.email", "test@example.invalid"]);
    git_run(&root, &["config", "user.name", "Test"]);
    write(&root, OWNED_SOURCE_PATHS[0], "accepted\n");
    commit(&root, "accepted source");
    let source = identity(&root);

    write(&root, "docs/checkpoint.md", "registry checkpoint\n");
    commit(&root, "metadata descendant");
    let metadata_head = git_output(&root, &["rev-parse", "HEAD"]);
    let context =
        crate::context::LiveContext::build(crate::context::BuildRequest::new(&root)).unwrap();
    let reads = context.begin_read_session().unwrap();
    assert!(verify_source(&reads, &source, &metadata_head).is_ok());

    write(&root, OWNED_SOURCE_PATHS[0], "changed authority\n");
    commit(&root, "changed authority");
    let changed_head = git_output(&root, &["rev-parse", "HEAD"]);
    let changed_context =
        crate::context::LiveContext::build(crate::context::BuildRequest::new(&root)).unwrap();
    let changed_reads = changed_context.begin_read_session().unwrap();
    assert!(verify_source(&changed_reads, &source, &changed_head).is_err());

    let mut wrong_tree = source;
    wrong_tree.tree = "0".repeat(40);
    assert!(verify_source(&reads, &wrong_tree, &metadata_head).is_err());
    fs::remove_dir_all(root).unwrap();
}

fn test_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "ultragoal-claim-source-{label}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&root).unwrap();
    root.canonicalize().unwrap()
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
    git_run(root, &["add", "."]);
    git_run(root, &["commit", "-q", "-m", message]);
}

fn git_run(root: &Path, args: &[&str]) {
    assert!(
        Command::new(protected_git())
            .arg("-C")
            .arg(root)
            .args(args)
            .status()
            .unwrap()
            .success()
    );
}

fn git_output(root: &Path, args: &[&str]) -> String {
    let output = Command::new(protected_git())
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn protected_git() -> &'static Path {
    ["/usr/bin/git", "/bin/git"]
        .into_iter()
        .map(Path::new)
        .find(|path| path.is_file())
        .expect("protected Git substrate")
}
