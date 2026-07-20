use crate::distribution::ConfinedRoot;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

pub(super) struct IsolatedRoot {
    path: PathBuf,
    confined: ConfinedRoot,
}

impl IsolatedRoot {
    pub(super) fn create() -> Result<Self, ()> {
        let parent = std::env::var_os("CODEX_WORKTREE_TMP")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .canonicalize()
            .map_err(|_| ())?;
        for _ in 0..16 {
            let name = format!(
                "hul-distribution-install-test-{}-{}",
                std::process::id(),
                NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
            );
            let path = parent.join(name);
            if create_private(&path).is_err() {
                continue;
            }
            return ConfinedRoot::open(&path)
                .map(|confined| Self { path, confined })
                .map_err(|_| ());
        }
        Err(())
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn confined(&self) -> ConfinedRoot {
        self.confined.clone()
    }

    pub(super) fn remove(self) -> Result<(), ()> {
        std::fs::remove_dir_all(self.path).map_err(|_| ())
    }
}

#[cfg(unix)]
fn create_private(path: &Path) -> Result<(), ()> {
    use std::os::unix::fs::DirBuilderExt;
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(path)
        .map_err(|_| ())
}

#[cfg(not(unix))]
fn create_private(_path: &Path) -> Result<(), ()> {
    Err(())
}
