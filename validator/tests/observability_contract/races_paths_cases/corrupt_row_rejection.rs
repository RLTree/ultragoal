use super::*;

#[test]
pub(crate) fn complete_corrupt_rows_are_not_silently_recovered() {
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

pub(crate) const RELATIVE_CWD_CHILD_ENV: &str = "HUL_OBSERVABILITY_RELATIVE_CWD_CHILD";

#[test]
pub(crate) fn relative_cwd_anchor_replacement_fails_closed_for_full_lifecycle() {
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
pub(crate) fn relative_cwd_anchor_replacement_child() {
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

pub(crate) fn relative_bound(path: impl AsRef<Path>) -> EventStore {
    EventStore::open_bound(path.as_ref(), "ctx-1", "cand-1", "source-1").unwrap()
}

pub(crate) fn write_replacement(path: &Path) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, b"replacement\n").unwrap();
}

pub(crate) fn assert_relative_rejected<T>(result: Result<T, String>) {
    let error = result.err().expect("replaced CWD anchor must fail closed");
    assert!(error.contains("ancestor substitution"), "{error}");
}
