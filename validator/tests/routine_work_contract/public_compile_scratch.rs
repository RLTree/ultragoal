use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_CLAIM: AtomicU64 = AtomicU64::new(0);

pub(crate) struct OwnedCompileScratch {
    path: PathBuf,
    device: u64,
    inode: u64,
}

impl OwnedCompileScratch {
    pub(crate) fn claim(label: &str) -> Self {
        let root = configured_root("CODEX_WORKTREE_SCRATCH");
        for _ in 0..64 {
            let name = format!(
                "{label}-{}-{}",
                std::process::id(),
                NEXT_CLAIM.fetch_add(1, Ordering::Relaxed)
            );
            let path = root.join(name);
            match fs::create_dir(&path) {
                Ok(()) => {
                    let metadata = fs::metadata(&path).expect("compile scratch identity");
                    return Self {
                        path,
                        device: metadata.dev(),
                        inode: metadata.ino(),
                    };
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("compile scratch claim failed: {error}"),
            }
        }
        panic!("compile scratch claim collisions exhausted")
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn teardown_after_assertions(&mut self) {
        let metadata = fs::metadata(&self.path).expect("compile scratch remains present");
        assert_eq!(
            (metadata.dev(), metadata.ino()),
            (self.device, self.inode),
            "compile scratch identity changed"
        );
        fs::remove_dir_all(&self.path).expect("compile scratch teardown failed");
    }
}

pub(crate) fn configured_root(name: &str) -> PathBuf {
    let value = std::env::var_os(name).unwrap_or_else(|| panic!("{name} is required"));
    let path = fs::canonicalize(PathBuf::from(value))
        .unwrap_or_else(|error| panic!("{name} is unavailable: {error}"));
    assert!(path.is_absolute() && path.is_dir(), "invalid {name}");
    path
}
