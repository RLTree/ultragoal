use super::fixture::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use ultragoal::orchestration::product::{ProductError, ProductionRootAuthority};

#[test]
fn ledger_tampering_and_truncation_fail_closed_without_repair() {
    let (authority_root, _journal, permit, authority) = issued_fixture("ledger-bytes");
    let ledger_path = authority_root.path().join("replay-ledger.jsonl");
    let issued = fs::read(&ledger_path).unwrap();
    let forged = String::from_utf8(issued.clone())
        .unwrap()
        .replace("\"state\":\"issued\"", "\"state\":\"refused\"");
    fs::write(&ledger_path, forged).unwrap();
    let before = recursive_fingerprint(authority_root.path());
    assert_eq!(
        authority.replay_state(&permit).unwrap_err(),
        ProductError::AuthorityStoreInvalid
    );
    assert_eq!(
        ProductionRootAuthority::open_existing(authority_root.path(), root_actor()).unwrap_err(),
        ProductError::AuthorityStoreInvalid
    );
    assert_eq!(recursive_fingerprint(authority_root.path()), before);

    let (authority_root, _journal, permit, authority) = issued_fixture("ledger-truncated");
    let ledger_path = authority_root.path().join("replay-ledger.jsonl");
    let issued = fs::read(&ledger_path).unwrap();
    fs::write(&ledger_path, &issued[..issued.len() - 1]).unwrap();
    let before = recursive_fingerprint(authority_root.path());
    assert_eq!(
        authority.replay_state(&permit).unwrap_err(),
        ProductError::AuthorityStoreInvalid
    );
    assert_eq!(recursive_fingerprint(authority_root.path()), before);
}

#[test]
fn actor_hardlink_lock_and_root_substitution_fail_closed() {
    let (authority_root, _journal, permit, authority) = issued_fixture("actor-swap");
    fs::write(
        authority_root.path().join("root-actor"),
        b"corrupt-actor-record\n",
    )
    .unwrap();
    assert_eq!(
        authority.replay_state(&permit).unwrap_err(),
        ProductError::AuthorityStoreInvalid
    );

    let (authority_root, _journal, _permit, _authority) = issued_fixture("hardlink");
    let hardlink_root = TestRoot::new("hardlink-alias", 0o700);
    fs::hard_link(
        authority_root.path().join("authority-key"),
        hardlink_root.path().join("key-alias"),
    )
    .unwrap();
    assert_eq!(
        ProductionRootAuthority::open_existing(authority_root.path(), root_actor()).unwrap_err(),
        ProductError::AuthorityStoreInvalid
    );

    let (authority_root, _journal, permit, authority) = issued_fixture("lock-swap");
    let lock = authority_root.path().join("replay-lock");
    fs::rename(&lock, authority_root.path().join("displaced-lock")).unwrap();
    fs::write(&lock, b"").unwrap();
    fs::set_permissions(&lock, fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(
        authority.replay_state(&permit).unwrap_err(),
        ProductError::AuthorityStoreInvalid
    );

    let (authority_root, _journal, permit, authority) = issued_fixture("root-swap");
    let moved = authority_root.path().with_extension("moved");
    fs::rename(authority_root.path(), &moved).unwrap();
    fs::create_dir(authority_root.path()).unwrap();
    fs::set_permissions(authority_root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(
        authority.replay_state(&permit).unwrap_err(),
        ProductError::AuthorityStoreInvalid
    );
    fs::remove_dir(authority_root.path()).unwrap();
    fs::rename(moved, authority_root.path()).unwrap();
}

fn issued_fixture(
    label: &str,
) -> (
    TestRoot,
    TestRoot,
    ultragoal::orchestration::product::RootPermit,
    ProductionRootAuthority,
) {
    let journal = interrupted_root(&format!("{label}-journal"));
    let authority_root = TestRoot::new(&format!("{label}-authority"), 0o700);
    let (_, permit, authority) = issue_resume(&authority_root, &journal.0, journal.1, 2);
    (authority_root, journal.0, permit, authority)
}
