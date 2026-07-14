use std::fs::{self, OpenOptions};

use super::context::{BuildRequest, LiveContext};
use super::routine_work::{LocalDirtyTree, RoutineErrorId, set_test_live_authority_hook};
use super::scenario::TempRepo;

#[test]
fn clean_and_dirty_capture_are_recursively_zero_write() {
    for (label, dirty) in [("zero-write-clean", false), ("zero-write-dirty", true)] {
        let repo = TempRepo::new(label);
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
    }
}

#[test]
fn symlink_and_hardlink_entries_fail_closed() {
    let symlink_repo = TempRepo::new("symlink");
    #[cfg(unix)]
    std::os::unix::fs::symlink("lib.rs", symlink_repo.root().join("src/alias.rs")).unwrap();
    #[cfg(unix)]
    {
        let context = symlink_repo.context("routine");
        let error = LocalDirtyTree::capture(&context).unwrap_err();
        assert_eq!(error.id(), RoutineErrorId::UnsupportedEntry);
    }

    let hardlink_repo = TempRepo::new("hardlink");
    hardlink_repo.write("src/linked.rs", b"linked\n");
    fs::hard_link(
        hardlink_repo.root().join("src/linked.rs"),
        hardlink_repo.root().join("src/linked-again.rs"),
    )
    .unwrap();
    let context = hardlink_repo.context("routine");
    let error = LocalDirtyTree::capture(&context).unwrap_err();
    assert_eq!(error.id(), RoutineErrorId::UnsupportedEntry);
}

#[cfg(unix)]
#[test]
fn fifo_entry_fails_without_blocking_or_reading_it() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let repo = TempRepo::new("fifo");
    let fifo = repo.root().join("src/input.pipe");
    let name = CString::new(fifo.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    let context = repo.context("routine");
    let error = LocalDirtyTree::capture(&context).unwrap_err();
    assert_eq!(error.id(), RoutineErrorId::UnsupportedEntry);
}

#[cfg(unix)]
#[test]
fn unix_socket_entry_is_not_an_invisible_clean_candidate() {
    let repo = TempRepo::new("socket");
    let socket =
        std::os::unix::net::UnixListener::bind(repo.root().join("src/input.sock")).unwrap();
    let context = repo.context("routine");
    let error = LocalDirtyTree::capture(&context).unwrap_err();
    drop(socket);
    assert_eq!(error.id(), RoutineErrorId::UnsupportedEntry);
}

#[test]
fn oversized_dirty_file_is_bounded_before_hashing() {
    let repo = TempRepo::new("oversized");
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
}

#[test]
fn dirty_submodule_is_rejected_instead_of_misread_as_a_regular_file() {
    let source = TempRepo::new("submodule-source");
    let parent = TempRepo::new("submodule-parent");
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
}

#[cfg(unix)]
#[test]
fn root_alias_is_canonicalized_before_routine_capture() {
    let repo = TempRepo::new("root-alias");
    let alias = std::env::temp_dir().join(format!("hul-routine-alias-{}", std::process::id()));
    let _ = fs::remove_file(&alias);
    std::os::unix::fs::symlink(repo.root(), &alias).unwrap();
    let context = LiveContext::build(BuildRequest::new(&alias)).unwrap();
    fs::remove_file(alias).unwrap();
    assert_eq!(context.worktree_root(), repo.root().canonicalize().unwrap());
    assert!(LocalDirtyTree::capture(&context).is_ok());
}

#[test]
fn tracked_candidate_change_during_capture_is_a_concurrent_mutation() {
    let repo = TempRepo::new("capture-mutation");
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
}

#[test]
fn untracked_file_mutation_after_initial_validation_fails_closed() {
    let repo = TempRepo::new("live-file-mutation");
    let context = repo.context("routine");
    let root = context.worktree_root().to_path_buf();
    set_test_live_authority_hook(move || {
        fs::write(root.join("after-validation.txt"), b"new input\n").unwrap();
    });
    let error = LocalDirtyTree::capture(&context).unwrap_err();
    assert_eq!(error.id(), RoutineErrorId::ConcurrentMutation);
}

#[test]
fn live_root_swap_at_final_revalidation_fails_closed() {
    let repo = TempRepo::new("live-root-swap");
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
}
