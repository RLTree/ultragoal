use std::fs;
use std::io;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ALIAS: AtomicU64 = AtomicU64::new(0);

pub(crate) struct ConfiguredPathAlias {
    path: PathBuf,
}

impl ConfiguredPathAlias {
    pub(crate) fn claim(label: &str, target: &Path) -> Self {
        let root = std::env::var_os("CODEX_WORKTREE_TMP")
            .map(PathBuf::from)
            .unwrap_or_else(|| panic!("CODEX_WORKTREE_TMP is required"));
        let root = fs::canonicalize(root).expect("configured worktree tmp is unavailable");
        for _ in 0..64 {
            let nonce = NEXT_ALIAS.fetch_add(1, Ordering::Relaxed);
            let path = root.join(format!("{label}-{}-{nonce}", std::process::id()));
            match symlink(target, &path) {
                Ok(()) => {
                    return Self { path };
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("configured alias claim failed: {error}"),
            }
        }
        panic!("configured alias collisions exhausted")
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn child_from_current_dir(&self, child: &str) -> PathBuf {
        relative_path(&std::env::current_dir().unwrap(), &self.path).join(child)
    }

    // Explicit fixture teardown after all actors and assertions finish. This has no
    // same-UID mutation-safety claim and is never called from Drop.
    pub(crate) fn teardown_after_assertions(&mut self) {
        fs::remove_file(&self.path).expect("explicit configured alias teardown");
    }
}

fn relative_path(from: &Path, to: &Path) -> PathBuf {
    let from = from.components().collect::<Vec<_>>();
    let to = to.components().collect::<Vec<_>>();
    let shared = from
        .iter()
        .zip(&to)
        .take_while(|(left, right)| left == right)
        .count();
    assert!(shared > 0, "configured alias has no common root");
    let mut relative = PathBuf::new();
    for _ in shared..from.len() {
        relative.push("..");
    }
    for component in &to[shared..] {
        relative.push(component.as_os_str());
    }
    relative
}
