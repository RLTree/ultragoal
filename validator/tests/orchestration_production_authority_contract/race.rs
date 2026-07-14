use super::fixture::*;
use std::fs;
use std::process::Command;
use ultragoal::orchestration::product::{ProductError, ProductionRootAuthority};

#[test]
fn separate_processes_cannot_reopen_nonempty_state_without_external_custody() {
    if std::env::var_os(CHILD_ENV).is_some() {
        return;
    }
    let (journal, head) = interrupted_root("blocked-process-journal");
    let authority_root = TestRoot::new("blocked-process-authority", 0o700);
    let barrier = TestRoot::new("blocked-process-barrier", 0o700);
    let (_, _, authority) = issue_resume(&authority_root, &journal, head, 2);
    let input = ChildInput {
        authority_root: authority_root.path().to_path_buf(),
        barrier_root: barrier.path().to_path_buf(),
    };
    let input_path = barrier.path().join("input.json");
    fs::write(&input_path, serde_json::to_vec(&input).unwrap()).unwrap();
    let before = recursive_fingerprint(authority_root.path());
    drop(authority);
    let statuses = (0..2)
        .map(|_| {
            Command::new(std::env::current_exe().unwrap())
                .args(["--exact", CHILD_TEST, "--nocapture"])
                .env(CHILD_ENV, &input_path)
                .status()
                .unwrap()
        })
        .collect::<Vec<_>>();
    assert!(statuses.iter().all(|status| status.success()));
    let outcomes = fs::read_dir(barrier.path())
        .unwrap()
        .filter_map(|entry| {
            let path = entry.unwrap().path();
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("outcome-")
                .then(|| fs::read_to_string(path).unwrap())
        })
        .collect::<Vec<_>>();
    assert_eq!(outcomes, vec!["HUL-ORCH-PROD-017", "HUL-ORCH-PROD-017"]);
    assert_eq!(recursive_fingerprint(authority_root.path()), before);
}

#[test]
fn production_authority_child() {
    let Some(path) = std::env::var_os(CHILD_ENV) else {
        return;
    };
    let input: ChildInput = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let outcome = ProductionRootAuthority::open_existing(&input.authority_root, root_actor())
        .map(|_| "unexpected-open")
        .unwrap_err()
        .code();
    fs::write(
        input
            .barrier_root
            .join(format!("outcome-{}", std::process::id())),
        outcome,
    )
    .unwrap();
}
