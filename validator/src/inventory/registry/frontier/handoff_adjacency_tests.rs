use super::handoff_adjacency::{RECEIPT, verify};
use crate::context::{BuildRequest, LiveContext};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

#[test]
fn receipt_child_requires_exact_adjacency_and_package_exclusion() {
    let root = fixture();
    let base = git(&root, &["rev-parse", "HEAD"]);
    fs::create_dir_all(root.join("validator/src/evaluation")).unwrap();
    fs::write(root.join("validator/src/evaluation/journal.rs"), "source").unwrap();
    commit(&root, "source");
    let source = git(&root, &["rev-parse", "HEAD"]);
    let source_tree = git(&root, &["rev-parse", "HEAD^{tree}"]);
    fs::create_dir_all(root.join("validation_artifacts/worker-results")).unwrap();
    fs::write(root.join(RECEIPT), "receipt").unwrap();
    commit(&root, "receipt");
    let child = git(&root, &["rev-parse", "HEAD"]);
    let child_tree = git(&root, &["rev-parse", "HEAD^{tree}"]);
    let record = json!({"status":"ready","base_commit":base,"handoff":{"status":"verified","commit":source,"tree":source_tree},"receipt_child":{"status":"verified","path":RECEIPT,"commit":child,"tree":child_tree,"parent_commit":source,"parent_tree":source_tree,"source_commit":source,"source_tree":source_tree},"ready_receipt":RECEIPT});
    let context = LiveContext::build(BuildRequest::new(&root)).unwrap();
    let reads = context.begin_read_session().unwrap();
    verify(&reads, &root, &record).unwrap();
    let mut forged = record;
    forged["receipt_child"]["parent_commit"] = forged["base_commit"].clone();
    assert!(
        verify(&reads, &root, &forged)
            .unwrap_err()
            .to_string()
            .contains("not adjacent")
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn inherited_receipt_path_is_not_a_valid_source_handoff() {
    let root = fixture();
    fs::create_dir_all(root.join("validation_artifacts/worker-results")).unwrap();
    fs::write(root.join(RECEIPT), "old").unwrap();
    commit(&root, "old receipt");
    let base = git(&root, &["rev-parse", "HEAD"]);
    fs::create_dir_all(root.join("validator/src/evaluation")).unwrap();
    fs::write(root.join("validator/src/evaluation/journal.rs"), "source").unwrap();
    commit(&root, "source");
    let source = git(&root, &["rev-parse", "HEAD"]);
    let source_tree = git(&root, &["rev-parse", "HEAD^{tree}"]);
    fs::write(root.join(RECEIPT), "new").unwrap();
    commit(&root, "receipt");
    let child = git(&root, &["rev-parse", "HEAD"]);
    let child_tree = git(&root, &["rev-parse", "HEAD^{tree}"]);
    let record = json!({"status":"ready","base_commit":base,"handoff":{"status":"verified","commit":source,"tree":source_tree},"receipt_child":{"status":"verified","path":RECEIPT,"commit":child,"tree":child_tree,"parent_commit":source,"parent_tree":source_tree,"source_commit":source,"source_tree":source_tree},"ready_receipt":RECEIPT});
    let context = LiveContext::build(BuildRequest::new(&root)).unwrap();
    let reads = context.begin_read_session().unwrap();
    assert!(
        verify(&reads, &root, &record)
            .unwrap_err()
            .to_string()
            .contains("contains its receipt child")
    );
    fs::remove_dir_all(root).unwrap();
}

fn fixture() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "ultragoal-handoff-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&root).unwrap();
    for args in [
        ["init", "-q"].as_slice(),
        ["config", "user.email", "test@example.invalid"].as_slice(),
        ["config", "user.name", "Test"].as_slice(),
    ] {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(&root)
                .status()
                .unwrap()
                .success()
        );
    }
    fs::write(
        root.join("plugin-manifest-draft.json"),
        "{\"resources\":[]}",
    )
    .unwrap();
    commit(&root, "base");
    root
}
fn commit(root: &PathBuf, message: &str) {
    assert!(
        Command::new("git")
            .args(["add", "-A"])
            .current_dir(root)
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new("git")
            .args(["commit", "-q", "-m", message])
            .current_dir(root)
            .status()
            .unwrap()
            .success()
    );
}
fn git(root: &PathBuf, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}
