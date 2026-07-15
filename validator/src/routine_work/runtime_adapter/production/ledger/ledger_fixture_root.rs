use super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(1);

pub(super) struct TestRoot {
    pub(super) parent: PathBuf,
    authority: PathBuf,
}

impl TestRoot {
    pub(super) fn new(label: &str) -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let parent = manifest
            .parent()
            .expect("ledger fixture manifest has no workspace parent")
            .join("target");
        fs::create_dir_all(&parent).expect("ledger fixture target directory is unavailable");
        let parent = fs::canonicalize(parent)
            .expect("ledger fixture target directory is unavailable")
            .join(format!(
                "hul-routine-ledger-{label}-{}-{}",
                std::process::id(),
                NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
            ));
        let authority = parent.join("authority");
        fs::create_dir(&parent).unwrap();
        fs::create_dir(&authority).unwrap();
        fs::set_permissions(&authority, fs::Permissions::from_mode(0o700)).unwrap();
        Self { parent, authority }
    }

    pub(super) fn path(&self) -> &Path {
        &self.authority
    }

    pub(super) fn state(&self) -> Vec<u8> {
        fs::read(self.authority.join(STATE_NAME)).unwrap()
    }

    pub(super) fn teardown_after_assertions(&mut self) {
        fs::remove_dir_all(&self.parent).expect("ledger fixture teardown failed");
        assert!(
            !self.parent.exists(),
            "ledger fixture teardown retained scope: {}",
            self.parent.display()
        );
    }
}
