use super::digest::sha256_hex;
use super::{BuildRequest, LiveContext};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::UNIX_EPOCH;

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

static NEXT: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Eq, PartialEq)]
struct SnapshotRow {
    relative_path: PathBuf,
    kind: &'static str,
    byte_length: u64,
    content_sha256: String,
    unix_mode: Option<u32>,
    modified_ns: Option<u128>,
}

fn content(path: &Path, metadata: &fs::Metadata) -> Vec<u8> {
    if metadata.is_file() {
        fs::read(path).unwrap()
    } else if metadata.file_type().is_symlink() {
        let target = fs::read_link(path).unwrap();
        #[cfg(unix)]
        {
            target.as_os_str().as_bytes().to_vec()
        }
        #[cfg(not(unix))]
        {
            target.to_string_lossy().as_bytes().to_vec()
        }
    } else {
        Vec::new()
    }
}

fn walk(root: &Path, current: &Path, rows: &mut Vec<SnapshotRow>) {
    let mut entries = fs::read_dir(current)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    entries.sort();
    for path in entries {
        let metadata = fs::symlink_metadata(&path).unwrap();
        let bytes = content(&path, &metadata);
        let kind = if metadata.is_file() {
            "file"
        } else if metadata.is_dir() {
            "directory"
        } else if metadata.file_type().is_symlink() {
            "symlink"
        } else {
            "special"
        };
        #[cfg(unix)]
        let unix_mode = Some(metadata.permissions().mode());
        #[cfg(not(unix))]
        let unix_mode = None;
        rows.push(SnapshotRow {
            relative_path: path.strip_prefix(root).unwrap().to_path_buf(),
            kind,
            byte_length: metadata.len(),
            content_sha256: sha256_hex(&bytes),
            unix_mode,
            modified_ns: metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_nanos()),
        });
        if metadata.is_dir() {
            walk(root, &path, rows);
        }
    }
}

fn snapshot(root: &Path) -> Vec<SnapshotRow> {
    let mut rows = Vec::new();
    walk(root, root, &mut rows);
    rows
}

#[test]
fn build_and_revalidate_leave_worktree_and_git_bytes_unchanged() {
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "ultragoal-context-zero-write-{}-{sequence}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    for args in [
        vec!["init", "-q"],
        vec!["config", "user.email", "context@example.invalid"],
        vec!["config", "user.name", "Context Test"],
    ] {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(&root)
                .status()
                .unwrap()
                .success()
        );
    }
    fs::write(root.join("tracked.txt"), b"stable\n").unwrap();
    assert!(
        Command::new("git")
            .args(["add", "tracked.txt"])
            .current_dir(&root)
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new("git")
            .args(["commit", "-q", "-m", "fixture"])
            .current_dir(&root)
            .status()
            .unwrap()
            .success()
    );
    let before = snapshot(&root);
    let context = LiveContext::build(
        BuildRequest::new(&root)
            .select_input("tracked.txt")
            .probe_tool("definitely-missing-ultragoal-tool"),
    )
    .unwrap();
    context.revalidate().unwrap();
    let after = snapshot(&root);
    assert_eq!(before, after);
    let _ = fs::remove_dir_all(root);
}
