use super::fixture::*;
use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use ultragoal::orchestration::product::command::OrchestrationStateRequest;
use ultragoal::orchestration::product::runtime_adapter::{
    OrchestrationRuntimeAdapter, RuntimeActionRequest, RuntimeActionSource,
};
use ultragoal::orchestration::product::{PermitReplayState, ProductError, ProductionRootAuthority};
use ultragoal::orchestration::product::{ProductWorkspace, ResumeRequest};

#[test]
fn committed_live_replay_state_is_read_only_and_permit_reuse_refuses() {
    let (journal, head) = interrupted_root("reopen-journal");
    let authority_root = TestRoot::new("reopen-authority", 0o700);
    let (action, permit, authority) = issue_resume(&authority_root, &journal, head, 2);
    let context = context();
    let workspace = ProductWorkspace::open(journal.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let view = adapter
        .inspect_current(&OrchestrationStateRequest {
            expected_head: action.expected_head.clone(),
            tick: 2,
            live_workers: BTreeSet::new(),
        })
        .unwrap();
    let request = RuntimeActionRequest::Resume(ResumeRequest {
        expected_head: action.expected_head.clone(),
        tick: 2,
        live_workers: BTreeSet::new(),
        target: action.target.clone(),
    });
    adapter
        .execute_production_action(
            &authority,
            RuntimeActionSource::Current(&view),
            &action,
            &permit,
            &request,
        )
        .unwrap();
    let before = recursive_fingerprint(authority_root.path());
    assert_eq!(
        authority.replay_state(&permit).unwrap(),
        Some(PermitReplayState::Committed)
    );
    assert_eq!(recursive_fingerprint(authority_root.path()), before);
    assert_eq!(
        adapter
            .execute_production_action(
                &authority,
                RuntimeActionSource::Current(&view),
                &action,
                &permit,
                &request,
            )
            .unwrap_err(),
        ProductError::AuthorityReplay
    );
    drop(authority);
    assert_eq!(
        ProductionRootAuthority::open_existing(authority_root.path(), root_actor()).unwrap_err(),
        ProductError::AuthorityCheckpointRequired
    );
    assert_eq!(recursive_fingerprint(authority_root.path()), before);
}

#[test]
fn missing_wrong_mode_symlink_extra_and_key_mutation_fail_closed() {
    let missing = TestRoot::new("missing", 0o700);
    let before = recursive_fingerprint(missing.path());
    assert_eq!(
        ProductionRootAuthority::open_existing(missing.path(), root_actor()).unwrap_err(),
        ProductError::AuthorityStoreInvalid
    );
    assert_eq!(recursive_fingerprint(missing.path()), before);

    let wrong_mode = TestRoot::new("wrong-mode", 0o755);
    assert_eq!(
        ProductionRootAuthority::open_or_initialize(wrong_mode.path(), root_actor()).unwrap_err(),
        ProductError::AuthorityStoreInvalid
    );

    let authority_root = TestRoot::new("mutated", 0o700);
    ProductionRootAuthority::open_or_initialize(authority_root.path(), root_actor()).unwrap();
    fs::write(authority_root.path().join("unexpected"), b"x").unwrap();
    assert_eq!(
        ProductionRootAuthority::open_existing(authority_root.path(), root_actor()).unwrap_err(),
        ProductError::AuthorityStoreInvalid
    );
    fs::remove_file(authority_root.path().join("unexpected")).unwrap();
    fs::set_permissions(
        authority_root.path().join("authority-key"),
        fs::Permissions::from_mode(0o644),
    )
    .unwrap();
    assert_eq!(
        ProductionRootAuthority::open_existing(authority_root.path(), root_actor()).unwrap_err(),
        ProductError::AuthorityStoreInvalid
    );

    let link_root = TestRoot::new("link-root", 0o700);
    let target = link_root.path().join("target");
    fs::create_dir(&target).unwrap();
    fs::set_permissions(&target, fs::Permissions::from_mode(0o700)).unwrap();
    let link = link_root.path().join("authority");
    symlink(&target, &link).unwrap();
    assert_eq!(
        ProductionRootAuthority::open_or_initialize(&link, root_actor()).unwrap_err(),
        ProductError::AuthorityStoreInvalid
    );
}

#[test]
fn explicit_initializer_recovers_only_valid_empty_crash_prefixes() {
    for (label, entries) in [
        ("lock-only", vec![("replay-lock", Vec::new())]),
        (
            "empty-ledger",
            vec![
                ("replay-lock", Vec::new()),
                ("replay-ledger.jsonl", Vec::new()),
            ],
        ),
        (
            "ledger-key",
            vec![
                ("replay-lock", Vec::new()),
                ("replay-ledger.jsonl", Vec::new()),
                ("authority-key", vec![7; 32]),
            ],
        ),
    ] {
        let root = TestRoot::new(label, 0o700);
        for (name, bytes) in entries {
            write_owner_file(root.path(), name, &bytes);
        }
        ProductionRootAuthority::open_or_initialize(root.path(), root_actor()).unwrap();
        assert_eq!(authority_names(&root), expected_authority_names());
        ProductionRootAuthority::open_existing(root.path(), root_actor()).unwrap();
    }

    let missing_ledger = TestRoot::new("established-missing-ledger", 0o700);
    write_owner_file(missing_ledger.path(), "replay-lock", b"");
    write_owner_file(missing_ledger.path(), "authority-key", &[9; 32]);
    write_owner_file(missing_ledger.path(), "root-actor", b"established\n");
    let before = recursive_fingerprint(missing_ledger.path());
    assert_eq!(
        ProductionRootAuthority::open_or_initialize(missing_ledger.path(), root_actor())
            .unwrap_err(),
        ProductError::AuthorityStoreInvalid
    );
    assert_eq!(recursive_fingerprint(missing_ledger.path()), before);

    let nonempty = TestRoot::new("partial-nonempty-ledger", 0o700);
    write_owner_file(nonempty.path(), "replay-lock", b"");
    write_owner_file(nonempty.path(), "replay-ledger.jsonl", b"not-empty\n");
    let before = recursive_fingerprint(nonempty.path());
    assert_eq!(
        ProductionRootAuthority::open_or_initialize(nonempty.path(), root_actor()).unwrap_err(),
        ProductError::AuthorityStoreInvalid
    );
    assert_eq!(recursive_fingerprint(nonempty.path()), before);
}

fn write_owner_file(root: &std::path::Path, name: &str, bytes: &[u8]) {
    let path = root.join(name);
    fs::write(&path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

fn authority_names(root: &TestRoot) -> BTreeSet<String> {
    fs::read_dir(root.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect()
}

fn expected_authority_names() -> BTreeSet<String> {
    BTreeSet::from([
        "authority-key".to_owned(),
        "replay-ledger.jsonl".to_owned(),
        "replay-lock".to_owned(),
        "root-actor".to_owned(),
    ])
}
