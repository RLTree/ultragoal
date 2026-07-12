use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Eq, PartialEq)]
struct SnapshotRow {
    relative_path: PathBuf,
    kind: &'static str,
    byte_length: u64,
    content_sha256: String,
    unix_mode: u32,
    modified_seconds: i64,
    modified_nanoseconds: i64,
}

#[derive(Debug, Eq, PartialEq)]
pub(super) struct Observation {
    tree: Vec<SnapshotRow>,
    pub(super) status: Vec<u8>,
}

pub(super) fn observe(root: &Path) -> Observation {
    let mut tree = Vec::new();
    push_row(root, root, &mut tree);
    Observation {
        tree,
        status: git_status(root),
    }
}

fn push_row(root: &Path, path: &Path, rows: &mut Vec<SnapshotRow>) {
    let metadata = fs::symlink_metadata(path).expect("snapshot metadata");
    let kind = if metadata.is_file() {
        "file"
    } else if metadata.is_dir() {
        "directory"
    } else if metadata.file_type().is_symlink() {
        "symlink"
    } else {
        "special"
    };
    rows.push(SnapshotRow {
        relative_path: path
            .strip_prefix(root)
            .expect("snapshot relative")
            .to_path_buf(),
        kind,
        byte_length: metadata.len(),
        content_sha256: digest(&content(path, &metadata)),
        unix_mode: metadata.mode(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
    });
    if metadata.is_dir() {
        let mut entries = fs::read_dir(path)
            .expect("snapshot directory")
            .map(|entry| entry.expect("snapshot entry").path())
            .collect::<Vec<_>>();
        entries.sort();
        for entry in entries {
            push_row(root, &entry, rows);
        }
    }
}

fn content(path: &Path, metadata: &fs::Metadata) -> Vec<u8> {
    if metadata.is_file() {
        fs::read(path).expect("read snapshot file")
    } else if metadata.file_type().is_symlink() {
        fs::read_link(path)
            .expect("read snapshot link")
            .as_os_str()
            .as_bytes()
            .to_vec()
    } else {
        Vec::new()
    }
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn git_status(root: &Path) -> Vec<u8> {
    let output = Command::new("/usr/bin/git")
        .args(["status", "--porcelain=v1", "-z", "--untracked-files=all"])
        .env_clear()
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("PATH", "/usr/bin:/bin")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_TERMINAL_PROMPT", "0")
        .current_dir(root)
        .output()
        .expect("git status");
    assert!(output.status.success(), "git status failed: {output:?}");
    output.stdout
}
