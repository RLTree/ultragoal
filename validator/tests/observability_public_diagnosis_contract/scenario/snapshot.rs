use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Eq, PartialEq)]
struct SnapshotRow {
    relative: PathBuf,
    kind: &'static str,
    device: u64,
    inode: u64,
    links: u64,
    mode: u32,
    length: u64,
    digest: String,
    modified: (i64, i64),
    changed: (i64, i64),
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Observation {
    rows: Vec<SnapshotRow>,
    status: Vec<u8>,
}

pub(crate) fn observe(root: &Path) -> Observation {
    let status = git_status(root);
    let mut rows = Vec::new();
    visit(root, root, &mut rows);
    Observation { rows, status }
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
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success());
    output.stdout
}

fn visit(root: &Path, path: &Path, rows: &mut Vec<SnapshotRow>) {
    let metadata = fs::symlink_metadata(path).unwrap();
    let kind = if metadata.is_file() {
        "file"
    } else if metadata.is_dir() {
        "directory"
    } else if metadata.file_type().is_symlink() {
        "symlink"
    } else {
        "special"
    };
    let content = if metadata.is_file() {
        fs::read(path).unwrap()
    } else if metadata.file_type().is_symlink() {
        fs::read_link(path).unwrap().as_os_str().as_bytes().to_vec()
    } else {
        Vec::new()
    };
    rows.push(SnapshotRow {
        relative: path.strip_prefix(root).unwrap().to_path_buf(),
        kind,
        device: metadata.dev(),
        inode: metadata.ino(),
        links: metadata.nlink(),
        mode: metadata.permissions().mode(),
        length: metadata.len(),
        digest: format!("sha256:{:x}", Sha256::digest(content)),
        modified: (metadata.mtime(), metadata.mtime_nsec()),
        changed: (metadata.ctime(), metadata.ctime_nsec()),
    });
    if metadata.is_dir() {
        let mut entries = fs::read_dir(path)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        entries.sort();
        for entry in entries {
            visit(root, &entry, rows);
        }
    }
}
