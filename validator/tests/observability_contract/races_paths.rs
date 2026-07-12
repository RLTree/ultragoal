use super::observability::{EventQuery, EventStore, SemanticEvent};
use super::support::{TestDir, event, query, store};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Barrier};

#[cfg(unix)]
use std::os::unix::fs::{PermissionsExt, symlink};

#[cfg(unix)]
#[test]
fn symlink_hardlink_special_file_and_parent_symlink_paths_are_rejected() {
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
fn nested_ancestor_symlink_is_rejected_before_store_binding_or_creation() {
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
fn ancestor_substitution_after_binding_rejects_before_an_outside_store_is_created() {
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
fn absent_store_ordinary_ancestor_replacement_rejects_before_any_store_is_created() {
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
fn ordinary_ancestor_replacement_rejects_clear_and_recovery_before_mutation() {
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
fn path_substitution_and_permission_failure_are_detected_without_content_leakage() {
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
fn a_store_opened_before_creation_rejects_external_materialization() {
    let dir = TestDir::new("external-materialization");
    let unbound = store(&dir);
    fs::write(dir.store_path(), b"").unwrap();
    let error = unbound.query(&query()).unwrap_err();
    assert!(error.contains("materialized outside append"), "{error}");
}

#[test]
fn concurrent_writers_and_readers_preserve_complete_rows_and_stable_ordering() {
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

#[test]
fn complete_corrupt_rows_are_not_silently_recovered() {
    let dir = TestDir::new("no-corrupt-recovery");
    let store = store(&dir);
    store.append(&event("one", 1, 1, "fail")).unwrap();
    OpenOptions::new()
        .append(true)
        .open(dir.store_path())
        .unwrap();
    let mut text = fs::read_to_string(dir.store_path()).unwrap();
    text = text.replace("\"fail\"", "\"pass\"");
    fs::write(dir.store_path(), text).unwrap();
    assert!(store.recover_truncated_tail().is_err());
}

const RELATIVE_CWD_CHILD_ENV: &str = "HUL_OBSERVABILITY_RELATIVE_CWD_CHILD";

#[test]
fn relative_cwd_anchor_replacement_fails_closed_for_full_lifecycle() {
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "races_paths::relative_cwd_anchor_replacement_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(RELATIVE_CWD_CHILD_ENV, "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "child failed\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn relative_cwd_anchor_replacement_child() {
    if std::env::var_os(RELATIVE_CWD_CHILD_ENV).is_none() {
        return;
    }
    let dir = TestDir::new("relative-cwd-anchor");
    let original = dir.path().join("work");
    let retained = dir.path().join("work-retained");
    fs::create_dir(&original).unwrap();
    std::env::set_current_dir(&original).unwrap();

    fs::create_dir_all("create/nested").unwrap();
    let create = relative_bound("create/nested/events.jsonl");
    fs::create_dir_all("existing").unwrap();
    let seed = relative_bound("existing/events.jsonl");
    seed.append(&event("seed", 1, 1, "pass")).unwrap();
    let append = relative_bound("existing/events.jsonl");
    let query_store = relative_bound("existing/events.jsonl");
    let clear = relative_bound("existing/events.jsonl");

    fs::create_dir_all("recovery").unwrap();
    let recovery_seed = relative_bound("recovery/events.jsonl");
    recovery_seed
        .append(&event("recover", 1, 1, "fail"))
        .unwrap();
    OpenOptions::new()
        .append(true)
        .open("recovery/events.jsonl")
        .unwrap()
        .write_all(b"{\"row_version\"")
        .unwrap();
    let recovery = relative_bound("recovery/events.jsonl");

    fs::rename(&original, &retained).unwrap();
    fs::create_dir(&original).unwrap();
    fs::create_dir_all(original.join("create/nested")).unwrap();
    write_replacement(&original.join("existing/events.jsonl"));
    write_replacement(&original.join("recovery/events.jsonl"));
    let retained_existing = retained.join("existing/events.jsonl");
    let retained_recovery = retained.join("recovery/events.jsonl");
    let before_existing = fs::read(&retained_existing).unwrap();
    let before_recovery = fs::read(&retained_recovery).unwrap();

    assert_relative_rejected(create.append(&event("create", 2, 1, "pass")));
    assert_relative_rejected(append.append(&event("append", 2, 2, "pass")));
    assert_relative_rejected(query_store.query(&query()));
    assert_relative_rejected(clear.clear());
    assert_relative_rejected(recovery.recover_truncated_tail());

    assert!(!original.join("create/nested/events.jsonl").exists());
    assert!(!retained.join("create/nested/events.jsonl").exists());
    assert_eq!(fs::read(&retained_existing).unwrap(), before_existing);
    assert_eq!(fs::read(&retained_recovery).unwrap(), before_recovery);
    assert_eq!(
        fs::read(original.join("existing/events.jsonl")).unwrap(),
        b"replacement\n"
    );
    assert_eq!(
        fs::read(original.join("recovery/events.jsonl")).unwrap(),
        b"replacement\n"
    );
}

fn relative_bound(path: impl AsRef<Path>) -> EventStore {
    EventStore::open_bound(path.as_ref(), "ctx-1", "cand-1", "source-1").unwrap()
}

fn write_replacement(path: &Path) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, b"replacement\n").unwrap();
}

fn assert_relative_rejected<T>(result: Result<T, String>) {
    let error = result.err().expect("replaced CWD anchor must fail closed");
    assert!(error.contains("ancestor substitution"), "{error}");
}
