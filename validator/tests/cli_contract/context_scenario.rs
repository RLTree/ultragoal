use super::context::{BuildRequest, LiveContext};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

static NEXT_DIR: AtomicU64 = AtomicU64::new(0);
static SERIAL: Mutex<()> = Mutex::new(());

pub(crate) fn serial() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|error| error.into_inner())
}

pub(crate) struct TestDir {
    pub(crate) root: PathBuf,
}

impl TestDir {
    pub(crate) fn empty(label: &str) -> Self {
        let sequence = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "ultragoal-context-{label}-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    pub(crate) fn repo(label: &str) -> Self {
        let fixture = Self::empty(label);
        fixture.git(&["init", "-q"]);
        fixture.git(&["config", "user.email", "context@example.invalid"]);
        fixture.git(&["config", "user.name", "Context Test"]);
        fs::write(fixture.root.join("tracked.txt"), b"one\n").unwrap();
        fixture.git(&["add", "tracked.txt"]);
        fixture.git(&["commit", "-q", "-m", "fixture"]);
        fixture
    }

    fn git(&self, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(&self.root)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?}");
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub(crate) fn build(root: &Path) -> LiveContext {
    LiveContext::build(BuildRequest::new(root)).unwrap()
}
