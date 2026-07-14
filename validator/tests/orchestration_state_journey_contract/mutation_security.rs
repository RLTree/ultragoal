use super::state_fixture::*;
use crate::orchestration::product::command::*;
use crate::orchestration::product::*;
use crate::orchestration::*;
use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};

#[cfg(unix)]
use std::ffi::CString;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::symlink;

fn clean(label: &str) -> (TestRoot, JournalHead) {
    let (root, engine) = durable_engine(label, false);
    let head = engine.journal_head().unwrap().clone();
    (root, head)
}

#[cfg(unix)]
#[test]
fn path_escape_links_and_special_files_fail_closed() {
    let (root, head) = clean("path-security");
    let escaped = root.path().join("..").join("path-security-canary");
    assert_eq!(
        ProductWorkspace::open(&escaped).unwrap_err(),
        ProductError::InvalidWorkspacePath
    );

    let link = root.path().with_extension("link");
    symlink(root.path(), &link).unwrap();
    assert_eq!(
        ProductWorkspace::open(&link).unwrap_err(),
        ProductError::InvalidWorkspacePath
    );
    fs::remove_file(link).unwrap();

    let fifo = root.path().join("special-fifo");
    let fifo_c = CString::new(fifo.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo_c.as_ptr(), 0o600) }, 0);
    assert_eq!(
        ProductWorkspace::open(&fifo).unwrap_err(),
        ProductError::InvalidWorkspacePath
    );

    let outside = TestRoot::new("hardlink-holder");
    fs::hard_link(
        root.path().join("events.jsonl"),
        outside.path().join("events-link"),
    )
    .unwrap();
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let before_head = fs::read(root.path().join("head.json")).unwrap();
    assert!(matches!(
        inspect(
            &context(),
            &workspace,
            &OrchestrationStateRequest {
                expected_head: head,
                tick: 1,
                live_workers: BTreeSet::new(),
            },
        ),
        Err(ProductError::Kernel(OrchestrationError::JournalCorrupt))
    ));
    assert_eq!(
        fs::read(root.path().join("head.json")).unwrap(),
        before_head
    );
}

#[cfg(unix)]
#[test]
fn workspace_ancestor_swap_is_rejected_without_reading_or_echoing_substitute() {
    let (root, head) = clean("ancestor-swap-private-canary");
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let moved = root.path().with_extension("anchored");
    let outside = TestRoot::new("outside-private-canary");
    fs::rename(root.path(), &moved).unwrap();
    symlink(outside.path(), root.path()).unwrap();

    let error = inspect(
        &context(),
        &workspace,
        &OrchestrationStateRequest {
            expected_head: head,
            tick: 1,
            live_workers: BTreeSet::new(),
        },
    )
    .unwrap_err();
    assert_eq!(error, ProductError::WorkspaceChanged);
    assert!(!error.to_string().contains("private-canary"));
    assert!(fs::read_dir(outside.path()).unwrap().next().is_none());

    fs::remove_file(root.path()).unwrap();
    fs::rename(moved, root.path()).unwrap();
}

#[test]
fn oversized_journal_and_forged_action_never_write_or_echo_attacker_values() {
    let (root, head) = clean("oversized");
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    OpenOptions::new()
        .write(true)
        .open(root.path().join("events.jsonl"))
        .unwrap()
        .set_len(64 * 1024 * 1024 + 1)
        .unwrap();
    let before_len = fs::metadata(root.path().join("events.jsonl"))
        .unwrap()
        .len();
    let before_head = fs::read(root.path().join("head.json")).unwrap();
    let error = inspect(
        &context(),
        &workspace,
        &OrchestrationStateRequest {
            expected_head: head,
            tick: 1,
            live_workers: BTreeSet::new(),
        },
    )
    .unwrap_err();
    assert!(matches!(
        error,
        ProductError::Kernel(OrchestrationError::JournalCorrupt)
    ));
    assert_eq!(
        fs::metadata(root.path().join("events.jsonl"))
            .unwrap()
            .len(),
        before_len
    );
    assert_eq!(
        fs::read(root.path().join("head.json")).unwrap(),
        before_head
    );

    let (action_root, mut engine) = durable_engine("forged-action", false);
    engine.interrupt_root(1).unwrap();
    let action_head = engine.journal_head().unwrap().clone();
    drop(engine);
    let action_workspace = ProductWorkspace::open(action_root.path()).unwrap();
    let view = inspect(
        &context(),
        &action_workspace,
        &OrchestrationStateRequest {
            expected_head: action_head,
            tick: 2,
            live_workers: BTreeSet::new(),
        },
    )
    .unwrap();
    let before = recursive_fingerprint(action_root.path());
    let mut forged = view.root_action_requests[0].clone();
    forged.target.operation_id = Some("attacker-private-canary/../escape".to_owned());
    let error = forged.validate_for(&action_workspace).unwrap_err();
    assert_eq!(error, ProductError::AuthorityInvalid);
    assert!(!error.to_string().contains("attacker-private-canary"));

    let mut oversized_view = view.clone();
    oversized_view.snapshot.plan.ready = vec!["x".repeat(16 * 1024 * 1024)];
    let error = project(
        &oversized_view,
        &action_workspace,
        &CommandProjection::Inspect,
    )
    .unwrap_err();
    assert_eq!(
        error,
        ProductError::Kernel(OrchestrationError::ResourceLimit)
    );
    assert_eq!(recursive_fingerprint(action_root.path()), before);
}
