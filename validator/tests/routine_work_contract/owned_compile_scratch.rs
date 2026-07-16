use std::ffi::CString;
use std::fs::{self, File};
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use super::owned_compile_claim::{create_marker, random_claim_name};
use super::owned_compile_directory::open_directory_at;
use super::owned_compile_retention::{
    CleanupDirective, CleanupOutcome, CleanupStage, retain, retain_controlled,
};

pub(crate) enum CleanupState {
    Claimed,
    Retained,
    Refused,
    TeardownComplete,
}

pub(crate) struct OwnedCompileScratch {
    pub(crate) parent: File,
    pub(crate) directory: File,
    pub(crate) name: CString,
    pub(crate) path: PathBuf,
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) root_device: u64,
    pub(crate) root_inode: u64,
    pub(crate) marker_device: u64,
    pub(crate) marker_inode: u64,
    pub(crate) marker_mac: [u8; 32],
    pub(crate) secret: [u8; 32],
    pub(crate) cleanup_state: CleanupState,
}

impl OwnedCompileScratch {
    pub(crate) fn claim(label: &str) -> Self {
        let root = configured_root("CODEX_WORKTREE_SCRATCH");
        let parent = File::open(&root).expect("configured scratch opens");
        let root_metadata = parent.metadata().expect("configured scratch identity");
        for _ in 0..64 {
            let name = random_claim_name(label);
            if unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } != 0 {
                let error = io::Error::last_os_error();
                if error.kind() == io::ErrorKind::AlreadyExists {
                    continue;
                }
                panic!("compile scratch claim failed: {error}");
            }
            let directory = open_directory_at(parent.as_raw_fd(), &name).unwrap();
            let metadata = directory.metadata().unwrap();
            let mut secret = [0_u8; 32];
            getrandom::fill(&mut secret).expect("scratch claim secret unavailable");
            let marker = create_marker(
                directory.as_raw_fd(),
                root_metadata.dev(),
                root_metadata.ino(),
                &name,
                metadata.dev(),
                metadata.ino(),
                &secret,
            )
            .unwrap();
            let path = root.join(name.to_str().unwrap());
            return Self {
                parent,
                directory,
                name,
                path,
                device: metadata.dev(),
                inode: metadata.ino(),
                root_device: root_metadata.dev(),
                root_inode: root_metadata.ino(),
                marker_device: marker.device,
                marker_inode: marker.inode,
                marker_mac: marker.mac,
                secret,
                cleanup_state: CleanupState::Claimed,
            };
        }
        panic!("compile scratch claim collisions exhausted")
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn recover_interrupted(&mut self) -> CleanupOutcome {
        retain(self)
    }

    pub(crate) fn recover_with(
        &mut self,
        pause: impl FnMut(CleanupStage) -> CleanupDirective,
    ) -> CleanupOutcome {
        retain_controlled(self, pause)
    }

    // Explicit fixture teardown after all actors and assertions finish. This has no
    // same-UID mutation-safety claim and is never called by recovery or Drop.
    pub(crate) fn teardown_after_assertions(&mut self) {
        assert!(
            super::owned_compile_claim::authenticates_claim(self),
            "explicit teardown requires an uncontested exact claim"
        );
        fs::remove_dir_all(&self.path).expect("explicit post-assertion teardown failed");
        self.cleanup_state = CleanupState::TeardownComplete;
    }
}

pub(crate) fn configured_root(name: &str) -> PathBuf {
    let value = std::env::var_os(name).unwrap_or_else(|| panic!("{name} is required"));
    let path = fs::canonicalize(PathBuf::from(value))
        .unwrap_or_else(|error| panic!("{name} is unavailable: {error}"));
    assert!(path.is_absolute() && path.is_dir(), "invalid {name}");
    path
}
