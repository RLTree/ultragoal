use std::ffi::CString;
use std::fs::File;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

#[cfg(test)]
thread_local! {
    static RECONCILIATION_REFUSALS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static CAPTURE_REFUSALS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static CAPTURE_ATTEMPTS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn set_reconciliation_refusals(count: usize) {
    RECONCILIATION_REFUSALS.with(|value| value.set(count));
}

#[cfg(test)]
pub(crate) fn set_capture_identity_refusals(count: usize) {
    CAPTURE_REFUSALS.with(|value| value.set(count));
    CAPTURE_ATTEMPTS.with(|value| value.set(0));
}

#[cfg(test)]
pub(crate) fn capture_identity_attempts() -> usize {
    CAPTURE_ATTEMPTS.with(std::cell::Cell::get)
}

use super::scope::{ClaimedFixtureScope, FixtureScopeBinding, FixtureScopeError};

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ClaimIdentity {
    pub(crate) device: u64,
    pub(crate) inode: u64,
}

#[derive(Debug)]
pub(crate) struct UnopenedClaimResidue {
    pub(crate) parent: File,
    pub(crate) name: CString,
    pub(crate) path: PathBuf,
    pub(crate) identity: Option<ClaimIdentity>,
    pub(crate) error: FixtureScopeError,
}

#[derive(Debug)]
pub(crate) struct OpenedClaimResidue {
    pub(crate) parent: File,
    pub(crate) directory: File,
    pub(crate) name: CString,
    pub(crate) path: PathBuf,
    pub(crate) identity: ClaimIdentity,
    pub(crate) error: FixtureScopeError,
}

impl ClaimResidue {
    pub(crate) fn error(&self) -> &FixtureScopeError {
        match self {
            Self::Unopened(value) => &value.error,
            Self::Opened(value) => &value.error,
        }
    }

    pub(crate) fn reconcile(self) -> Result<ClaimedFixtureScope, Self> {
        match self {
            Self::Unopened(value) => value.reconcile(),
            Self::Opened(value) => value.reconcile().map_err(Self::Opened),
        }
    }
}

impl UnopenedClaimResidue {
    fn reconcile(mut self) -> Result<ClaimedFixtureScope, ClaimResidue> {
        if reconciliation_refused() {
            return Err(ClaimResidue::Unopened(self));
        }
        let identity = match self.identity {
            Some(identity) => identity,
            None => match capture_identity(&self.parent, &self.name) {
                Ok(identity) => {
                    self.identity = Some(identity);
                    identity
                }
                Err(error) => {
                    self.error = error;
                    return Err(ClaimResidue::Unopened(self));
                }
            },
        };
        let descriptor = unsafe {
            libc::openat(
                self.parent.as_raw_fd(),
                self.name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if descriptor < 0 {
            return Err(ClaimResidue::Unopened(self));
        }
        OpenedClaimResidue {
            parent: self.parent,
            directory: unsafe { File::from_raw_fd(descriptor) },
            name: self.name,
            path: self.path,
            identity,
            error: self.error,
        }
        .reconcile()
        .map_err(ClaimResidue::Opened)
    }
}

fn reconciliation_refused() -> bool {
    #[cfg(test)]
    {
        return RECONCILIATION_REFUSALS.with(|value| {
            let count = value.get();
            value.set(count.saturating_sub(1));
            count > 0
        });
    }
    #[cfg(not(test))]
    {
        false
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
        if named.st_dev as u64 != self.identity.device
            || named.st_ino as u64 != self.identity.inode
            || metadata.dev() != self.identity.device
            || metadata.ino() != self.identity.inode
        {
            return Err(self);
        }
        Ok(ClaimedFixtureScope {
            path: self.path,
            parent: self.parent,
            directory: self.directory,
            name: self.name,
            device: self.identity.device,
            inode: self.identity.inode,
            binding: FixtureScopeBinding::Bound,
        })
    }
}

pub(crate) fn capture_identity(
    parent: &File,
    name: &CString,
) -> Result<ClaimIdentity, FixtureScopeError> {
    if capture_refused() {
        return Err(FixtureScopeError::Retained(
            "injected claim identity capture refusal".to_owned(),
        ));
    }
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    if unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } != 0
    {
        return Err(FixtureScopeError::Retained(format!(
            "claim identity failed: {}",
            std::io::Error::last_os_error()
        )));
    }
    let stat = unsafe { stat.assume_init() };
    if stat.st_mode & libc::S_IFMT != libc::S_IFDIR {
        return Err(FixtureScopeError::Retained(
            "claimed child is not a directory".to_owned(),
        ));
    }
    Ok(ClaimIdentity {
        device: stat.st_dev as u64,
        inode: stat.st_ino as u64,
    })
}

fn capture_refused() -> bool {
    #[cfg(test)]
    {
        CAPTURE_ATTEMPTS.with(|value| value.set(value.get().saturating_add(1)));
        return CAPTURE_REFUSALS.with(|value| {
            let count = value.get();
            value.set(count.saturating_sub(1));
            count > 0
        });
    }
    #[cfg(not(test))]
    {
        false
    }
}
