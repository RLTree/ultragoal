use std::ffi::CString;
use std::fs::File;
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use crate::catalog_fixture_scope::{ClaimedFixtureScope, FixtureScopeBinding, FixtureScopeError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ClaimFailurePoint {
    AfterMkdirBeforeOpen,
    AfterOpenBeforeIdentity,
}

#[derive(Debug)]
pub(crate) enum FixtureClaimFailure {
    Failed(FixtureScopeError),
    Retained(ClaimResidue),
}

#[derive(Debug)]
pub(crate) enum ClaimResidue {
    Unopened(UnopenedClaimResidue),
    Opened(OpenedClaimResidue),
}

#[derive(Debug)]
pub(crate) struct UnopenedClaimResidue {
    pub(crate) parent: File,
    pub(crate) name: CString,
    pub(crate) path: PathBuf,
    pub(crate) error: FixtureScopeError,
}

#[derive(Debug)]
pub(crate) struct OpenedClaimResidue {
    pub(crate) parent: File,
    pub(crate) directory: File,
    pub(crate) name: CString,
    pub(crate) path: PathBuf,
    pub(crate) error: FixtureScopeError,
}

impl ClaimResidue {
    pub(crate) fn path(&self) -> &Path {
        match self {
            Self::Unopened(value) => &value.path,
            Self::Opened(value) => &value.path,
        }
    }

    pub(crate) fn error(&self) -> &FixtureScopeError {
        match self {
            Self::Unopened(value) => &value.error,
            Self::Opened(value) => &value.error,
        }
    }

    pub(crate) fn reconcile(self) -> Result<ClaimedFixtureScope, Self> {
        match self {
            Self::Unopened(value) => Err(Self::Unopened(value)),
            Self::Opened(value) => value.reconcile().map_err(Self::Opened),
        }
    }
}

impl OpenedClaimResidue {
    pub(crate) fn reconcile(self) -> Result<ClaimedFixtureScope, Self> {
        let metadata = match self.directory.metadata() {
            Ok(value) if value.file_type().is_dir() => value,
            _ => return Err(self),
        };
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        if unsafe {
            libc::fstatat(
                self.parent.as_raw_fd(),
                self.name.as_ptr(),
                stat.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } != 0
        {
            return Err(self);
        }
        let named = unsafe { stat.assume_init() };
        if named.st_dev as u64 != metadata.dev() || named.st_ino as u64 != metadata.ino() {
            return Err(self);
        }
        Ok(ClaimedFixtureScope {
            path: self.path,
            parent: self.parent,
            directory: self.directory,
            name: self.name,
            device: metadata.dev(),
            inode: metadata.ino(),
            binding: FixtureScopeBinding::Bound,
        })
    }
}
