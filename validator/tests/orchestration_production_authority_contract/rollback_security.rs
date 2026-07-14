use super::fixture::*;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::MetadataExt;
use std::thread;
use std::time::Duration;
use ultragoal::orchestration::product::{ProductError, ProductionRootAuthority};

#[test]
fn same_inode_valid_prefix_and_mutate_restore_cannot_rollback_committed_tail() {
    let (journal, head) = interrupted_root("same-inode-committed-journal");
    let authority_root = TestRoot::new("same-inode-committed-authority", 0o700);
    let (action, permit, authority) = issue_resume(&authority_root, &journal, head, 2);
    let ledger = authority_root.path().join("replay-ledger.jsonl");
    let issued_prefix = fs::read(&ledger).unwrap();
    execute_resume(journal.path(), &action, &permit, &authority, 2).unwrap();
    let committed_bytes = fs::read(&ledger).unwrap();
    rewrite_same_inode(&ledger, &issued_prefix);
    let before = recursive_fingerprint(authority_root.path());
    assert_eq!(
        authority.replay_state(&permit).unwrap_err(),
        ProductError::AuthorityCheckpointRequired
    );
    assert_eq!(
        ProductionRootAuthority::open_existing(authority_root.path(), root_actor()).unwrap_err(),
        ProductError::AuthorityCheckpointRequired
    );
    assert_eq!(recursive_fingerprint(authority_root.path()), before);

    append_stale_suffix(&ledger, &committed_bytes[issued_prefix.len()..]);
    let before = recursive_fingerprint(authority_root.path());
    assert_eq!(
        authority.replay_state(&permit).unwrap_err(),
        ProductError::AuthorityCheckpointRequired
    );
    assert_eq!(recursive_fingerprint(authority_root.path()), before);
}

#[test]
fn concurrent_nonempty_reopens_refuse_without_mutation_or_replay() {
    let (journal, head) = interrupted_root("same-inode-reserved-journal");
    let authority_root = TestRoot::new("same-inode-reserved-authority", 0o700);
    let (_, _, authority) = issue_resume(&authority_root, &journal, head, 2);
    let before = recursive_fingerprint(authority_root.path());
    drop(authority);
    let root = authority_root.path().to_path_buf();
    let results = (0..4)
        .map(|_| {
            let root = root.clone();
            thread::spawn(move || {
                ProductionRootAuthority::open_existing(&root, root_actor()).unwrap_err()
            })
        })
        .map(|child| child.join().unwrap())
        .collect::<Vec<_>>();
    assert!(
        results
            .iter()
            .all(|error| *error == ProductError::AuthorityCheckpointRequired)
    );
    assert_eq!(recursive_fingerprint(authority_root.path()), before);
}

fn rewrite_same_inode(path: &std::path::Path, bytes: &[u8]) {
    let before = fs::metadata(path).unwrap();
    thread::sleep(Duration::from_millis(2));
    fs::write(path, bytes).unwrap();
    let after = fs::metadata(path).unwrap();
    assert_eq!((before.dev(), before.ino()), (after.dev(), after.ino()));
    assert_ne!(
        (before.ctime(), before.ctime_nsec()),
        (after.ctime(), after.ctime_nsec())
    );
}

fn append_stale_suffix(path: &std::path::Path, bytes: &[u8]) {
    let before = fs::metadata(path).unwrap();
    thread::sleep(Duration::from_millis(2));
    let mut file = OpenOptions::new().append(true).open(path).unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
    let after = fs::metadata(path).unwrap();
    assert_eq!((before.dev(), before.ino()), (after.dev(), after.ino()));
    assert_ne!(
        (before.ctime(), before.ctime_nsec()),
        (after.ctime(), after.ctime_nsec())
    );
}
