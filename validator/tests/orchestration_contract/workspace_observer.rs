use super::durable_journal_fixture::*;
use super::orchestration_fixture::*;
use crate::orchestration::*;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;

fn sha(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn intent(
    prior: BTreeMap<String, Option<String>>,
    expected: BTreeMap<String, String>,
) -> RootIntegrationIntent {
    RootIntegrationIntent {
        schema_version: "RootIntegrationIntent-v1".to_owned(),
        base_binding: binding(),
        root_actor: "ultra-root".to_owned(),
        accepted_proposals: BTreeMap::from([("lease-001".to_owned(), digest('8'))]),
        prior_digests: prior,
        expected_digests: expected,
    }
}

fn tree(root: &JournalRoot) -> BTreeMap<String, Vec<u8>> {
    fn visit(base: &std::path::Path, at: &std::path::Path, rows: &mut BTreeMap<String, Vec<u8>>) {
        let mut entries: Vec<_> = fs::read_dir(at)
            .unwrap()
            .map(|entry| entry.unwrap())
            .collect();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let relative = path
                .strip_prefix(base)
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned();
            if entry.file_type().unwrap().is_dir() {
                visit(base, &path, rows);
            } else {
                rows.insert(relative, fs::read(path).unwrap());
            }
        }
    }
    let mut rows = BTreeMap::new();
    visit(root.path(), root.path(), &mut rows);
    rows
}

#[test]
fn live_workspace_observation_classifies_none_partial_conflict_and_full() {
    let root = JournalRoot::new("workspace-observer-states");
    fs::create_dir(root.path()).unwrap();
    fs::write(root.path().join("a.txt"), b"old-a").unwrap();
    fs::write(root.path().join("b.txt"), b"old-b").unwrap();
    let intent = intent(
        BTreeMap::from([
            ("a.txt".to_owned(), Some(sha(b"old-a"))),
            ("b.txt".to_owned(), Some(sha(b"old-b"))),
        ]),
        BTreeMap::from([
            ("a.txt".to_owned(), sha(b"new-a")),
            ("b.txt".to_owned(), sha(b"new-b")),
        ]),
    );
    let workspace = RootWorkspace::open(root.path()).unwrap();
    let before = tree(&root);
    let none = workspace.observe(&intent).unwrap();
    assert_eq!(none.disposition, IntegrationDisposition::None);
    assert_eq!(tree(&root), before);

    fs::write(root.path().join("a.txt"), b"new-a").unwrap();
    assert_eq!(
        workspace.observe(&intent).unwrap().disposition,
        IntegrationDisposition::Partial
    );
    fs::write(root.path().join("b.txt"), b"foreign").unwrap();
    assert_eq!(
        workspace.observe(&intent).unwrap().disposition,
        IntegrationDisposition::Conflict
    );
    fs::write(root.path().join("b.txt"), b"new-b").unwrap();
    assert_eq!(
        workspace.observe(&intent).unwrap().disposition,
        IntegrationDisposition::Full
    );
}

#[test]
fn missing_files_are_observed_without_creation() {
    let root = JournalRoot::new("workspace-observer-missing");
    fs::create_dir(root.path()).unwrap();
    let intent = intent(
        BTreeMap::from([("created.txt".to_owned(), None)]),
        BTreeMap::from([("created.txt".to_owned(), sha(b"created"))]),
    );
    let workspace = RootWorkspace::open(root.path()).unwrap();
    let before = tree(&root);
    let observation = workspace.observe(&intent).unwrap();
    assert_eq!(observation.disposition, IntegrationDisposition::None);
    assert_eq!(tree(&root), before);
    assert!(!root.path().join("created.txt").exists());
}

#[cfg(unix)]
#[test]
fn aliases_links_special_files_and_root_swaps_fail_closed() {
    use std::os::unix::fs::symlink;

    let root = JournalRoot::new("workspace-observer-adversarial");
    fs::create_dir(root.path()).unwrap();
    fs::write(root.path().join("Target.txt"), b"value").unwrap();
    let alias_intent = intent(
        BTreeMap::from([("target.txt".to_owned(), Some(sha(b"value")))]),
        BTreeMap::from([("target.txt".to_owned(), sha(b"next"))]),
    );
    let workspace = RootWorkspace::open(root.path()).unwrap();
    assert_eq!(
        workspace.observe(&alias_intent).unwrap_err(),
        OrchestrationError::IntegrationAmbiguous
    );

    let exact_intent = intent(
        BTreeMap::from([("Target.txt".to_owned(), Some(sha(b"value")))]),
        BTreeMap::from([("Target.txt".to_owned(), sha(b"next"))]),
    );
    fs::hard_link(
        root.path().join("Target.txt"),
        root.path().join("hardlink.txt"),
    )
    .unwrap();
    assert!(workspace.observe(&exact_intent).is_err());
    fs::remove_file(root.path().join("hardlink.txt")).unwrap();

    fs::remove_file(root.path().join("Target.txt")).unwrap();
    symlink("outside", root.path().join("Target.txt")).unwrap();
    assert!(workspace.observe(&exact_intent).is_err());

    fs::remove_file(root.path().join("Target.txt")).unwrap();
    let fifo = std::ffi::CString::new(root.path().join("Target.txt").to_str().unwrap()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    assert!(workspace.observe(&exact_intent).is_err());

    let moved = root.path().with_extension("moved");
    fs::rename(root.path(), &moved).unwrap();
    fs::create_dir(root.path()).unwrap();
    assert!(workspace.observe(&exact_intent).is_err());
    fs::remove_dir(root.path()).unwrap();
    fs::rename(moved, root.path()).unwrap();
}
