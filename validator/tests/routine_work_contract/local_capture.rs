use super::configured_path_alias::ConfiguredPathAlias;
use super::context::{BuildRequest, LiveContext};
use super::routine_work::{LocalDirtyTree, RoutineErrorId, set_test_live_authority_hook};
use super::scenario::TempRepo;
use std::fs::{self, OpenOptions};

#[test]
fn clean_and_dirty_capture_are_recursively_zero_write() {
    for (label, dirty) in [("zero-write-clean", false), ("zero-write-dirty", true)] {
        let mut repo = TempRepo::new(label);
        if dirty {
            repo.write("src/lib.rs", b"pub fn value() -> u8 { 9 }\n");
        }
        let context = repo.context("routine");
        let tree_before = repo.tree();
        let status_before = repo.status();
        let snapshot = LocalDirtyTree::capture(&context).unwrap();
        assert_eq!(snapshot.is_clean(), !dirty);
        assert_eq!(repo.status(), status_before);
        assert_eq!(repo.tree(), tree_before);
        repo.teardown_after_assertions();
    }
}

#[test]
fn symlink_and_hardlink_entries_fail_closed() {
    let mut symlink_repo = TempRepo::new("symlink");
    #[cfg(unix)]
    std::os::unix::fs::symlink("lib.rs", symlink_repo.root().join("src/alias.rs")).unwrap();
    #[cfg(unix)]
    {
        let context = symlink_repo.context("routine");
        let error = LocalDirtyTree::capture(&context).unwrap_err();
        assert_eq!(error.id(), RoutineErrorId::UnsupportedEntry);
    }

    let mut hardlink_repo = TempRepo::new("hardlink");
    hardlink_repo.write("src/linked.rs", b"linked\n");
    fs::hard_link(
        hardlink_repo.root().join("src/linked.rs"),
        hardlink_repo.root().join("src/linked-again.rs"),
    )
    .unwrap();
    let context = hardlink_repo.context("routine");
    let error = LocalDirtyTree::capture(&context).unwrap_err();
    assert_eq!(error.id(), RoutineErrorId::UnsupportedEntry);
    symlink_repo.teardown_after_assertions();
    hardlink_repo.teardown_after_assertions();
}

#[cfg(unix)]
#[test]
fn fifo_entry_fails_without_blocking_or_reading_it() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let mut repo = TempRepo::new("fifo");
    let fifo = repo.root().join("src/input.pipe");
    let name = CString::new(fifo.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    let context = repo.context("routine");
    let error = LocalDirtyTree::capture(&context).unwrap_err();
    assert_eq!(error.id(), RoutineErrorId::UnsupportedEntry);
    repo.teardown_after_assertions();
}

#[cfg(unix)]
#[test]
fn unix_socket_entry_is_not_an_invisible_clean_candidate() {
    let mut repo = TempRepo::new("socket");
    let mut alias = ConfiguredPathAlias::claim("routine-capture-socket", &repo.root().join("src"));
    let bind_path = alias.child_from_current_dir("input.sock");
    let socket = std::os::unix::net::UnixListener::bind(&bind_path).unwrap();
    let context = repo.context("routine");
    let error = LocalDirtyTree::capture(&context).unwrap_err();
    assert_eq!(error.id(), RoutineErrorId::UnsupportedEntry);
    drop(socket);
    alias.teardown_after_assertions();
    repo.teardown_after_assertions();
}

#[test]
fn oversized_dirty_file_is_bounded_before_hashing() {
    let mut repo = TempRepo::new("oversized");
    let path = repo.root().join("src/oversized.bin");
    OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .unwrap()
        .set_len(16 * 1024 * 1024 + 1)
        .unwrap();
    let context = repo.context("routine");
    let error = LocalDirtyTree::capture(&context).unwrap_err();
    assert_eq!(error.id(), RoutineErrorId::CaptureLimit);
    repo.teardown_after_assertions();
}

#[test]
fn dirty_submodule_is_rejected_instead_of_misread_as_a_regular_file() {
    let mut source = TempRepo::new("submodule-source");
    let mut parent = TempRepo::new("submodule-parent");
    let source_path = source.root().to_str().unwrap();
    parent.git(&[
        "-c",
        "protocol.file.allow=always",
        "submodule",
        "add",
        "-q",
        source_path,
        "vendor/sub",
    ]);
    parent.git(&["add", "."]);
    parent.git(&["commit", "-q", "-m", "add submodule"]);
    parent.write("vendor/sub/src/lib.rs", b"dirty submodule\n");
    let context = parent.context("routine");
    let error = LocalDirtyTree::capture(&context).unwrap_err();
    assert_eq!(error.id(), RoutineErrorId::UnsupportedEntry);
    parent.teardown_after_assertions();
    source.teardown_after_assertions();
}

#[cfg(unix)]
#[test]
fn root_alias_is_canonicalized_before_routine_capture() {
    let mut repo = TempRepo::new("root-alias");
    let mut alias = ConfiguredPathAlias::claim("routine-capture-root", repo.root());
    let context = LiveContext::build(BuildRequest::new(alias.path())).unwrap();
    assert_eq!(context.worktree_root(), repo.root().canonicalize().unwrap());
    assert!(LocalDirtyTree::capture(&context).is_ok());
    alias.teardown_after_assertions();
    repo.teardown_after_assertions();
}

#[test]
fn tracked_candidate_change_during_capture_is_a_concurrent_mutation() {
    let mut repo = TempRepo::new("capture-mutation");
    let context = repo.context("routine");
    let root = repo.root().to_path_buf();
    set_test_live_authority_hook(move || {
        fs::write(
            root.join("src/lib.rs"),
            b"changed after initial validation\n",
        )
        .unwrap();
    });
    let error = LocalDirtyTree::capture(&context).unwrap_err();
    assert_eq!(error.id(), RoutineErrorId::ConcurrentMutation);
    repo.teardown_after_assertions();
}

#[test]
fn untracked_file_mutation_after_initial_validation_fails_closed() {
    let mut repo = TempRepo::new("live-file-mutation");
    let context = repo.context("routine");
    let root = context.worktree_root().to_path_buf();
    set_test_live_authority_hook(move || {
        fs::write(root.join("after-validation.txt"), b"new input\n").unwrap();
    });
    let error = LocalDirtyTree::capture(&context).unwrap_err();
    assert_eq!(error.id(), RoutineErrorId::ConcurrentMutation);
    repo.teardown_after_assertions();
}

#[test]
fn live_root_swap_at_final_revalidation_fails_closed() {
    let mut repo = TempRepo::new("live-root-swap");
    let context = repo.context("routine");
    let root = context.worktree_root().to_path_buf();
    let backup = root.with_extension(format!("routine-swap-{}", std::process::id()));
    let swap_root = root.clone();
    let swap_backup = backup.clone();
    set_test_live_authority_hook(move || {
        fs::rename(&swap_root, &swap_backup).unwrap();
        fs::create_dir(&swap_root).unwrap();
    });
    let error = LocalDirtyTree::capture(&context).unwrap_err();
    fs::remove_dir(&root).unwrap();
    fs::rename(&backup, &root).unwrap();
    assert!(matches!(
        error.id(),
        RoutineErrorId::CaptureFailed | RoutineErrorId::ConcurrentMutation
    ));
    repo.teardown_after_assertions();
}
