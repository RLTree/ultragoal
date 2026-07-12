use super::super::artifact::{self, ReadStage};
use std::{cell::Cell, ffi::CString, os::unix::ffi::OsStrExt, path::PathBuf};

const PATH_INVALID: &str = "review_round_anchor_path_invalid";
const CHANGED: &str = "review_round_anchor_changed_during_read";
const NOT_REGULAR: &str = "review_round_anchor_not_regular";

#[test]
fn rejects_ordinary_directory_substitution_before_ancestor_open() {
    let root = root("ordinary-ancestor-swap");
    let outside = root.with_extension("outside");
    write(root.join("fixtures/anchor.json"), "trusted");
    write(outside.join("anchor.json"), "escaped");

    let result = artifact::read_with_hook(&root, "fixtures/anchor.json", |stage| {
        if stage == ReadStage::BeforeAncestorOpen(0) {
            std::fs::rename(root.join("fixtures"), root.join("held")).expect("hold trusted");
            std::fs::rename(&outside, root.join("fixtures")).expect("substitute ancestor");
        }
    });

    assert_error(result, CHANGED);
    std::fs::remove_dir_all(root).expect("cleanup root");
}

#[test]
fn rejects_ordinary_leaf_substitution_before_leaf_open() {
    let root = root("ordinary-leaf-swap");
    let outside = root.with_extension("outside.json");
    write(root.join("fixtures/anchor.json"), "trusted");
    write(&outside, "escaped");

    let result = artifact::read_with_hook(&root, "fixtures/anchor.json", |stage| {
        if stage == ReadStage::BeforeLeafOpen {
            std::fs::rename(root.join("fixtures/anchor.json"), root.join("held.json"))
                .expect("hold trusted");
            std::fs::rename(&outside, root.join("fixtures/anchor.json")).expect("substitute leaf");
        }
    });

    assert_error(result, CHANGED);
    std::fs::remove_dir_all(root).expect("cleanup root");
}

#[test]
fn rejects_over_byte_and_over_depth_paths_before_open_hooks() {
    let root = root("path-bounds");
    write(root.join("fixtures/anchor.json"), "trusted");
    let hook_calls = Cell::new(0);
    let over_bytes = "a".repeat(artifact::MAX_RELATIVE_PATH_BYTES + 1);
    let result = artifact::read_with_hook(&root, &over_bytes, |_| hook_calls.set(1));
    assert_error(result, PATH_INVALID);
    assert_eq!(hook_calls.get(), 0);

    let over_depth = std::iter::repeat_n("a", artifact::MAX_COMPONENTS + 1)
        .chain(std::iter::once("anchor.json"))
        .collect::<Vec<_>>()
        .join("/");
    let result = artifact::read_with_hook(&root, &over_depth, |_| hook_calls.set(1));
    assert_error(result, PATH_INVALID);
    assert_eq!(hook_calls.get(), 0);
    std::fs::remove_dir_all(root).expect("cleanup root");
}

#[test]
fn rejects_fifo_without_blocking_or_echoing() {
    let root = root("fifo");
    let path = root.join("fixtures/SECRET_FIFO_CANARY.json");
    std::fs::create_dir_all(path.parent().expect("parent")).expect("fixtures");
    let path_bytes = CString::new(path.as_os_str().as_bytes()).expect("fifo path");
    let status = unsafe { libc::mkfifo(path_bytes.as_ptr(), 0o600) };
    assert_eq!(status, 0, "mkfifo failed");

    let error = read_error(&root, "fixtures/SECRET_FIFO_CANARY.json");
    assert_eq!(error, NOT_REGULAR);
    assert!(!error.contains("SECRET_FIFO_CANARY"));
    std::fs::remove_dir_all(root).expect("cleanup root");
}

#[test]
fn rejects_unix_socket_without_blocking_or_echoing() {
    use std::os::unix::net::UnixListener;

    let root = root("socket");
    let path = root.join("fixtures/SECRET_SOCKET_CANARY.json");
    std::fs::create_dir_all(path.parent().expect("parent")).expect("fixtures");
    let listener = UnixListener::bind(&path).expect("bind socket");

    let error = read_error(&root, "fixtures/SECRET_SOCKET_CANARY.json");
    assert_eq!(error, NOT_REGULAR);
    assert!(!error.contains("SECRET_SOCKET_CANARY"));
    drop(listener);
    std::fs::remove_dir_all(root).expect("cleanup root");
}

fn root(label: &str) -> PathBuf {
    let root = PathBuf::from("/tmp").join(format!("n02-anchor005-{label}-{}", std::process::id()));
    if root.exists() {
        std::fs::remove_dir_all(&root).expect("remove stale root");
    }
    std::fs::create_dir_all(&root).expect("create root");
    root.canonicalize().expect("canonical root")
}

fn write(path: impl AsRef<std::path::Path>, status: &str) {
    let path = path.as_ref();
    std::fs::create_dir_all(path.parent().expect("parent")).expect("parent");
    std::fs::write(path, format!(r#"{{"status":"{status}"}}"#)).expect("write JSON");
}

fn read_error(root: &std::path::Path, relative: &str) -> String {
    match artifact::read(root, relative) {
        Ok(_) => panic!("untrusted anchor was accepted"),
        Err(error) => error,
    }
}

fn assert_error(result: Result<artifact::JsonArtifact, String>, expected: &str) {
    assert_eq!(
        match result {
            Ok(_) => panic!("untrusted anchor was accepted"),
            Err(error) => error,
        },
        expected
    );
}
