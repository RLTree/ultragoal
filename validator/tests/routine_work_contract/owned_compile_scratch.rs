use std::ffi::CString;
use std::fs::{self, File};
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use super::owned_compile_claim::{create_marker, random_claim_name};
use super::owned_compile_custody::ForeignCustody;
use super::owned_compile_directory::open_directory_at;
use super::owned_compile_quarantine::{
    CleanupDirective, CleanupOutcome, CleanupStage, cleanup, cleanup_controlled,
};

pub(crate) const FAILURE_MARKER: &[u8] = b"compile scratch removed after induced failure\n";

pub(crate) enum CleanupState {
    Claimed,
    Quarantined(CString),
    Cleared(CString),
    DisplacedForeign {
        quarantine: CString,
        custody: ForeignCustody,
        destination: Option<(u64, u64)>,
    },
    Settled,
}

pub(crate) struct OwnedCompileScratch {
    pub(crate) parent: File,
    pub(crate) directory: File,
    pub(crate) name: CString,
    pub(crate) path: PathBuf,
    pub(crate) failure_marker: PathBuf,
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
            let failure_marker = root.join(format!("{}.failure.txt", name.to_str().unwrap()));
            return Self {
                parent,
                directory,
                name,
                path,
                failure_marker,
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

    pub(crate) fn failure_marker(&self) -> &Path {
        &self.failure_marker
    }

    pub(crate) fn recovery_path(&self) -> Option<PathBuf> {
        match &self.cleanup_state {
            CleanupState::Quarantined(name) | CleanupState::Cleared(name) => {
                Some(self.path.parent().unwrap().join(name.to_str().unwrap()))
            }
            CleanupState::DisplacedForeign { custody, .. } => custody
                .current_name(self.path.parent().unwrap())
                .map(|name| self.path.parent().unwrap().join(name.to_str().unwrap())),
            CleanupState::Claimed | CleanupState::Settled => None,
        }
    }

    pub(crate) fn ambiguous_destination(&self) -> Option<Option<(u64, u64)>> {
        match self.cleanup_state {
            CleanupState::DisplacedForeign { destination, .. } => Some(destination),
            _ => None,
        }
    }

    pub(crate) fn recover_interrupted(&mut self) -> CleanupOutcome {
        cleanup(self)
    }

    pub(crate) fn recover_with(
        &mut self,
        pause: impl FnMut(CleanupStage) -> CleanupDirective,
    ) -> CleanupOutcome {
        cleanup_controlled(self, pause)
    }
}

impl Drop for OwnedCompileScratch {
    fn drop(&mut self) {
        if !matches!(self.cleanup_state, CleanupState::Settled) {
            let _ = cleanup(self);
        }
    }
}

pub(crate) fn configured_root(name: &str) -> PathBuf {
    let value = std::env::var_os(name).unwrap_or_else(|| panic!("{name} is required"));
    let path = fs::canonicalize(PathBuf::from(value))
        .unwrap_or_else(|error| panic!("{name} is unavailable: {error}"));
    assert!(path.is_absolute() && path.is_dir(), "invalid {name}");
    path
}
