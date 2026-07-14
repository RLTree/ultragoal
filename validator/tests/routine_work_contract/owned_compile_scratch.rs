use std::ffi::CString;
use std::fs::{self, File};
use std::io;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::owned_compile_quarantine::{cleanup, open_directory_at, write_new_file};

static NEXT_SCRATCH: AtomicU64 = AtomicU64::new(0);
pub(crate) const FAILURE_MARKER: &[u8] = b"compile scratch removed after induced failure\n";
pub(crate) const SUBSTITUTION_MARKER: &[u8] =
    b"renamed compile scratch retained after path substitution\n";
pub(crate) const SUBSTITUTION_MARKER_NAME: &str = "RETAINED-SUBSTITUTION.txt";

pub(crate) struct OwnedCompileScratch {
    pub(crate) parent: File,
    pub(crate) directory: File,
    pub(crate) name: CString,
    pub(crate) path: PathBuf,
    pub(crate) failure_marker: PathBuf,
    pub(crate) device: u64,
    pub(crate) inode: u64,
}

impl OwnedCompileScratch {
    pub(crate) fn claim(label: &str) -> Self {
        let root = configured_root("CODEX_WORKTREE_SCRATCH");
        let _ = configured_root("CODEX_WORKTREE_TMP");
        let parent = File::open(&root).expect("configured scratch opens");
        for _ in 0..64 {
            let nonce = NEXT_SCRATCH.fetch_add(1, Ordering::Relaxed);
            let name = CString::new(format!("{label}-{}-{nonce}", std::process::id())).unwrap();
            if unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } != 0 {
                let error = io::Error::last_os_error();
                if error.kind() == io::ErrorKind::AlreadyExists {
                    continue;
                }
                panic!("compile scratch claim failed: {error}");
            }
            let directory = open_directory_at(parent.as_raw_fd(), &name).unwrap();
            write_new_file(
                directory.as_raw_fd(),
                "OWNERSHIP.txt",
                b"descriptor-owned compile scratch\n",
            )
            .unwrap();
            let metadata = directory.metadata().unwrap();
            let path = root.join(name.to_str().unwrap());
            let failure_marker = root.join(format!("{}.failure.txt", name.to_str().unwrap()));
            return Self {
                parent,
                directory,
                name,
                path,
                failure_marker,
                device: metadata.dev(),
                inode: metadata.ino(),
            };
        }
        panic!("compile scratch claim collisions exhausted")
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn failure_marker(&self) -> &Path {
        &self.failure_marker
    }

    pub(crate) fn reclaim_interrupted(path: &Path) {
        let root = configured_root("CODEX_WORKTREE_SCRATCH");
        assert_eq!(path.parent(), Some(root.as_path()));
        let name = CString::new(path.file_name().unwrap().as_encoded_bytes()).unwrap();
        let parent = File::open(&root).unwrap();
        let directory = open_directory_at(parent.as_raw_fd(), &name).unwrap();
        let metadata = directory.metadata().unwrap();
        drop(Self {
            parent,
            directory,
            name,
            path: path.to_path_buf(),
            failure_marker: path.with_extension("failure.txt"),
            device: metadata.dev(),
            inode: metadata.ino(),
        });
    }
}

impl Drop for OwnedCompileScratch {
    fn drop(&mut self) {
        cleanup(self);
    }
}

fn configured_root(name: &str) -> PathBuf {
    let value = std::env::var_os(name).unwrap_or_else(|| panic!("{name} is required"));
    let path = fs::canonicalize(PathBuf::from(value))
        .unwrap_or_else(|error| panic!("{name} is unavailable: {error}"));
    assert!(path.is_absolute() && path.is_dir(), "invalid {name}");
    path
}
