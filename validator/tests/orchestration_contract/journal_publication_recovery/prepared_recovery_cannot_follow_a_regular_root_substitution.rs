#[test]
fn prepared_recovery_cannot_follow_a_regular_root_substitution() {
    let (root, prior, log, event) = initialized("prepared-root-substitution");
    publish_interrupted_manually(&root, &log, &event);
    let prepared =
        FileJournal::prepare_interrupted_append(root.path(), &prior, &event.event_id, &binding())
            .unwrap();
    let names = ["events.jsonl", "head.json", "journal.lock"];
    let original_before = names.map(|name| fs::read(root.path().join(name)).unwrap());
    let moved = root.path().with_extension("anchored");
    fs::rename(root.path(), &moved).unwrap();
    fs::create_dir(root.path()).unwrap();
    for name in names {
        fs::copy(moved.join(name), root.path().join(name)).unwrap();
    }
    let replacement_before = names.map(|name| fs::read(root.path().join(name)).unwrap());

    assert_eq!(
        prepared.commit().unwrap_err(),
        OrchestrationError::JournalCorrupt
    );
    for (index, name) in names.into_iter().enumerate() {
        assert_eq!(fs::read(moved.join(name)).unwrap(), original_before[index]);
        assert_eq!(
            fs::read(root.path().join(name)).unwrap(),
            replacement_before[index]
        );
    }

    fs::remove_dir_all(root.path()).unwrap();
    fs::rename(moved, root.path()).unwrap();
}

#[cfg(unix)]
#[test]
fn recovery_opener_rejects_links_fifo_and_root_substitution() {
    use std::os::unix::fs::symlink;

    for kind in ["symlink", "hardlink", "fifo"] {
        let (root, prior, log, event) = initialized(kind);
        publish_interrupted_manually(&root, &log, &event);
        let events = root.path().join("events.jsonl");
        let backup = root.path().join("events.backup");
        fs::rename(&events, &backup).unwrap();
        match kind {
            "symlink" => symlink("events.backup", &events).unwrap(),
            "hardlink" => fs::hard_link(&backup, &events).unwrap(),
            "fifo" => {
                let path = std::ffi::CString::new(events.to_str().unwrap()).unwrap();
                assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
            }
            _ => unreachable!(),
        }
        assert!(
            FileJournal::recover_interrupted_append(
                root.path(),
                &prior,
                &event.event_id,
                &binding(),
            )
            .is_err()
        );
    }

    let (root, prior, log, event) = initialized("root-substitution");
    publish_interrupted_manually(&root, &log, &event);
    let moved = root.path().with_extension("moved");
    fs::rename(root.path(), &moved).unwrap();
    symlink(&moved, root.path()).unwrap();
    assert!(
        FileJournal::recover_interrupted_append(root.path(), &prior, &event.event_id, &binding(),)
            .is_err()
    );
    fs::remove_file(root.path()).unwrap();
    fs::rename(moved, root.path()).unwrap();
}
