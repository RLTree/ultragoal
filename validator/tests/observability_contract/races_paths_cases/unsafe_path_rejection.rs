use super::*;

#[cfg(unix)]
#[test]
pub(crate) fn symlink_hardlink_special_file_and_parent_symlink_paths_are_rejected() {
    let dir = TestDir::new("unsafe-paths");
    let real = dir.path().join("real.jsonl");
    fs::write(&real, b"").unwrap();

    let symlink_path = dir.path().join("symlink.jsonl");
    symlink(&real, &symlink_path).unwrap();
    assert!(EventStore::open_bound(&symlink_path, "ctx-1", "cand-1", "source-1").is_err());

    let hardlink_path = dir.path().join("hardlink.jsonl");
    fs::hard_link(&real, &hardlink_path).unwrap();
    assert!(EventStore::open_bound(&real, "ctx-1", "cand-1", "source-1").is_err());

    let special_path = dir.path().join("special");
    fs::create_dir(&special_path).unwrap();
    assert!(EventStore::open_bound(&special_path, "ctx-1", "cand-1", "source-1").is_err());

    let real_parent = dir.path().join("real-parent");
    fs::create_dir(&real_parent).unwrap();
    let linked_parent = dir.path().join("linked-parent");
    symlink(&real_parent, &linked_parent).unwrap();
    assert!(
        EventStore::open_bound(
            linked_parent.join("events.jsonl"),
            "ctx-1",
            "cand-1",
            "source-1"
        )
        .is_err()
    );
}

#[cfg(unix)]
#[test]
pub(crate) fn nested_ancestor_symlink_is_rejected_before_store_binding_or_creation() {
    let dir = TestDir::new("nested-ancestor-symlink");
    let real_nested = dir.path().join("real/nested");
    let outer = dir.path().join("outer");
    fs::create_dir_all(&real_nested).unwrap();
    fs::create_dir(&outer).unwrap();
    symlink(dir.path().join("real"), outer.join("link")).unwrap();

    let escaped_store = outer.join("link/nested/events.jsonl");
    let error = EventStore::open_bound(&escaped_store, "ctx-1", "cand-1", "source-1").unwrap_err();
    assert!(error.contains("ancestor symlink"), "{error}");
    assert!(!real_nested.join("events.jsonl").exists());
}

#[cfg(unix)]
#[test]
pub(crate) fn ancestor_substitution_after_binding_rejects_before_an_outside_store_is_created() {
    let dir = TestDir::new("ancestor-substitution");
    let original_nested = dir.path().join("outer/nested");
    let outside_nested = dir.path().join("outside/nested");
    fs::create_dir_all(&original_nested).unwrap();
    fs::create_dir_all(&outside_nested).unwrap();
    let store_path = original_nested.join("events.jsonl");
    let bound = EventStore::open_bound(&store_path, "ctx-1", "cand-1", "source-1").unwrap();

    fs::remove_dir_all(dir.path().join("outer")).unwrap();
    symlink(dir.path().join("outside"), dir.path().join("outer")).unwrap();
    let error = bound.append(&event("one", 1, 1, "pass")).unwrap_err();
    assert!(error.contains("ancestor symlink"), "{error}");
    assert!(!outside_nested.join("events.jsonl").exists());
}

#[cfg(unix)]
#[test]
pub(crate) fn absent_store_ordinary_ancestor_replacement_rejects_before_any_store_is_created() {
    let dir = TestDir::new("absent-store-ordinary-ancestor-replacement");
    let bound_nested = dir.path().join("outer/nested");
    let retained_outer = dir.path().join("outer-retained");
    fs::create_dir_all(&bound_nested).unwrap();
    let bound = EventStore::open_bound(
        bound_nested.join("events.jsonl"),
        "ctx-1",
        "cand-1",
        "source-1",
    )
    .unwrap();

    fs::rename(dir.path().join("outer"), &retained_outer).unwrap();
    fs::create_dir_all(&bound_nested).unwrap();
    let error = bound.append(&event("one", 1, 1, "pass")).unwrap_err();

    assert!(error.contains("ancestor substitution"), "{error}");
    assert!(
        bound
            .query(&query())
            .unwrap_err()
            .contains("ancestor substitution"),
        "read must reject the replaced ancestor before opening the replacement path"
    );
    assert!(
        !bound_nested.join("events.jsonl").exists(),
        "replacement path must not receive the first store"
    );
    assert!(
        !retained_outer.join("nested/events.jsonl").exists(),
        "the detached retained parent must not retain a rolled-forward store"
    );
}

#[cfg(unix)]
#[test]
pub(crate) fn ordinary_ancestor_replacement_rejects_clear_and_recovery_before_mutation() {
    let dir = TestDir::new("ordinary-ancestor-lifecycle-replacement");
    let nested = dir.path().join("outer/nested");
    let retained_outer = dir.path().join("outer-retained");
    let store_path = nested.join("events.jsonl");
    fs::create_dir_all(&nested).unwrap();
    let initial = EventStore::open_bound(&store_path, "ctx-1", "cand-1", "source-1").unwrap();
    initial.append(&event("one", 1, 1, "pass")).unwrap();
    OpenOptions::new()
        .append(true)
        .open(&store_path)
        .unwrap()
        .write_all(b"{\"row_version\"")
        .unwrap();
    let bound = EventStore::open_bound(&store_path, "ctx-1", "cand-1", "source-1").unwrap();

    fs::rename(dir.path().join("outer"), &retained_outer).unwrap();
    fs::create_dir_all(&nested).unwrap();
    let replacement = nested.join("events.jsonl");
    fs::write(&replacement, b"replacement\n").unwrap();
    let replacement_before = fs::read(&replacement).unwrap();
    let retained = retained_outer.join("nested/events.jsonl");
    let retained_before = fs::read(&retained).unwrap();

    let clear = bound.clear().unwrap_err();
    let recover = bound.recover_truncated_tail().unwrap_err();
    assert!(clear.contains("ancestor substitution"), "{clear}");
    assert!(recover.contains("ancestor substitution"), "{recover}");
    assert_eq!(fs::read(&replacement).unwrap(), replacement_before);
    assert_eq!(fs::read(&retained).unwrap(), retained_before);
}

#[cfg(unix)]
#[test]
pub(crate) fn path_substitution_and_permission_failure_are_detected_without_content_leakage() {
    let dir = TestDir::new("substitution");
    let initial_store = store(&dir);
    initial_store.append(&event("one", 1, 1, "pass")).unwrap();
    let bound = store(&dir);
    fs::remove_file(dir.store_path()).unwrap();
    fs::write(dir.store_path(), b"").unwrap();
    assert!(
        bound
            .query(&query())
            .unwrap_err()
            .contains("path substitution")
    );

    let permission_dir = TestDir::new("permission");
    let initial_store = store(&permission_dir);
    initial_store.append(&event("one", 1, 1, "pass")).unwrap();
    let mut permissions = fs::metadata(permission_dir.store_path())
        .unwrap()
        .permissions();
    permissions.set_mode(0o000);
    fs::set_permissions(permission_dir.store_path(), permissions).unwrap();
    let error = store(&permission_dir).query(&query()).unwrap_err();
    let mut restore = fs::metadata(permission_dir.store_path())
        .unwrap()
        .permissions();
    restore.set_mode(0o600);
    fs::set_permissions(permission_dir.store_path(), restore).unwrap();
    assert!(
        error.contains("permission-denied"),
        "safe permission error: {error}"
    );
    assert!(!error.contains(permission_dir.path().to_string_lossy().as_ref()));
}

#[test]
pub(crate) fn a_store_opened_before_creation_rejects_external_materialization() {
    let dir = TestDir::new("external-materialization");
    let unbound = store(&dir);
    fs::write(dir.store_path(), b"").unwrap();
    let error = unbound.query(&query()).unwrap_err();
    assert!(error.contains("materialized outside append"), "{error}");
}

#[test]
pub(crate) fn concurrent_writers_and_readers_preserve_complete_rows_and_stable_ordering() {
    let dir = TestDir::new("concurrent");
    let store = Arc::new(store(&dir));
    let barrier = Arc::new(Barrier::new(12));
    let mut threads = Vec::new();

    for worker in 0..8_u64 {
        let store = Arc::clone(&store);
        let barrier = Arc::clone(&barrier);
        threads.push(std::thread::spawn(move || {
            barrier.wait();
            for item in 0..8_u64 {
                let id = format!("event-{worker}-{item}");
                let event = SemanticEvent::new(
                    "ctx-1",
                    "cand-1",
                    "source-1",
                    id,
                    worker * 10 + item,
                    item,
                    "check.run",
                    "pass",
                )
                .unwrap();
                store.append(&event).unwrap();
            }
        }));
    }
    for _ in 0..4 {
        let store = Arc::clone(&store);
        let barrier = Arc::clone(&barrier);
        threads.push(std::thread::spawn(move || {
            barrier.wait();
            for _ in 0..16 {
                let _ = store.query(&EventQuery::new("ctx-1", "cand-1", "source-1").unwrap());
                std::thread::yield_now();
            }
        }));
    }
    for thread in threads {
        thread.join().unwrap();
    }

    let rows = store.query(&query()).unwrap();
    assert_eq!(rows.len(), 64);
    assert!(rows.windows(2).all(|pair| {
        (
            pair[0].observed_at_unix_ms(),
            pair[0].sequence(),
            pair[0].event_id(),
        ) <= (
            pair[1].observed_at_unix_ms(),
            pair[1].sequence(),
            pair[1].event_id(),
        )
    }));
    let persisted = fs::read(dir.store_path()).unwrap();
    assert!(persisted.ends_with(b"\n"));
    assert_eq!(persisted.iter().filter(|byte| **byte == b'\n').count(), 64);
}
