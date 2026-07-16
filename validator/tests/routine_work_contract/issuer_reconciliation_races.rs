use std::ffi::CString;
use std::fs;
use std::os::unix::fs::{FileTypeExt, symlink};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};

use super::owned_compile_retention::{CleanupDirective, CleanupOutcome, CleanupStage};
use super::owned_compile_scratch::OwnedCompileScratch;

#[test]
fn foreign_recovery_boundary_never_renames_a_replacement() {
    let mut owned = OwnedCompileScratch::claim("routine-retained-foreign");
    let original = owned.path().to_path_buf();
    let held = original.with_extension("foreign-held");
    fs::write(original.join("owned"), b"owned root\n").unwrap();
    let outcome = owned.recover_with(|stage| {
        if stage == CleanupStage::ForeignRecoveryRenameRetired {
            fs::rename(&original, &held).unwrap();
            fs::create_dir(&original).unwrap();
            fs::write(original.join("foreign"), b"foreign root\n").unwrap();
        }
        CleanupDirective::Continue
    });
    assert_eq!(outcome, CleanupOutcome::RefusedZeroWrite);
    assert_eq!(fs::read(held.join("owned")).unwrap(), b"owned root\n");
    assert_eq!(
        fs::read(original.join("foreign")).unwrap(),
        b"foreign root\n"
    );
    fs::remove_dir_all(original).unwrap();
    fs::remove_dir_all(held).unwrap();
}

#[test]
fn symlink_fifo_and_socket_entries_are_retained_without_traversal() {
    let mut owned = OwnedCompileScratch::claim("routine-retained-special");
    let symlink_path = owned.path().join("link");
    let fifo_path = owned.path().join("fifo");
    let socket_path = owned.path().join("socket");
    symlink("missing-target", &symlink_path).unwrap();
    let fifo = CString::new(fifo_path.as_os_str().as_encoded_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    let socket_relative = relative_path(&std::env::current_dir().unwrap(), &socket_path);
    let listener = UnixListener::bind(socket_relative).unwrap();

    assert_eq!(
        owned.recover_interrupted(),
        CleanupOutcome::RetainedNoDestructiveAuthority
    );
    assert!(
        fs::symlink_metadata(&symlink_path)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(
        fs::symlink_metadata(&fifo_path)
            .unwrap()
            .file_type()
            .is_fifo()
    );
    assert!(
        fs::symlink_metadata(&socket_path)
            .unwrap()
            .file_type()
            .is_socket()
    );
    drop(listener);
    owned.teardown_after_assertions();
}

fn relative_path(from: &Path, to: &Path) -> PathBuf {
    let from = from.components().collect::<Vec<_>>();
    let to = to.components().collect::<Vec<_>>();
    let shared = from
        .iter()
        .zip(&to)
        .take_while(|(left, right)| left == right)
        .count();
    let mut relative = PathBuf::new();
    for _ in shared..from.len() {
        relative.push("..");
    }
    for component in &to[shared..] {
        relative.push(component.as_os_str());
    }
    relative
}

#[test]
fn special_entry_substitution_at_retired_unlink_preserves_both_names() {
    let mut owned = OwnedCompileScratch::claim("routine-retained-special-swap");
    let entry = owned.path().join("entry");
    let held = owned.path().join("entry-held");
    symlink("owned-target", &entry).unwrap();
    let outcome = owned.recover_with(|stage| {
        if stage == CleanupStage::DescendantEntryUnlinkRetired {
            fs::rename(&entry, &held).unwrap();
            let fifo = CString::new(entry.as_os_str().as_encoded_bytes()).unwrap();
            assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
        }
        CleanupDirective::Continue
    });
    assert_eq!(outcome, CleanupOutcome::RetainedNoDestructiveAuthority);
    assert!(
        fs::symlink_metadata(&held)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(fs::symlink_metadata(&entry).unwrap().file_type().is_fifo());
    owned.teardown_after_assertions();
}
