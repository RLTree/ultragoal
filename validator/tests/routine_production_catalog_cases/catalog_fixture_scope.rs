use std::ffi::CString;
use std::fs::{self, File};
use std::io;
use std::os::fd::{AsRawFd, FromRawFd};
use std::path::{Path, PathBuf};

use crate::catalog_fixture_claim::{
    ClaimFailurePoint, ClaimResidue, FixtureClaimFailure, OpenedClaimResidue, UnopenedClaimResidue,
    capture_identity,
};

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum FixtureScopeError {
    Claim(String),
    Setup(&'static str),
    Retained(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CatalogSetupFailurePoint {
    AfterClaim,
    AfterDirectories,
    AfterCatalogWrite,
}

pub(crate) struct ClaimedFixtureScope {
    pub(super) path: PathBuf,
    pub(super) parent: File,
    pub(super) directory: File,
    pub(super) name: CString,
    pub(super) device: u64,
    pub(super) inode: u64,
    pub(super) binding: FixtureScopeBinding,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum FixtureScopeBinding {
    Bound,
    Detached,
}

impl ClaimedFixtureScope {
    pub(crate) fn claim(parent: &Path, child: &str) -> Result<Self, FixtureClaimFailure> {
        Self::claim_with_failure(parent, child, None)
    }

    pub(crate) fn claim_with_failure(
        parent: &Path,
        child: &str,
        failure: Option<ClaimFailurePoint>,
    ) -> Result<Self, FixtureClaimFailure> {
        fs::create_dir_all(parent).map_err(|error| {
            FixtureClaimFailure::Failed(FixtureScopeError::Claim(format!("parent: {error}")))
        })?;
        let parent_directory = File::open(parent).map_err(|error| {
            FixtureClaimFailure::Failed(FixtureScopeError::Claim(format!("parent open: {error}")))
        })?;
        let path = parent.join(child);
        Self::claim_from_parent(parent_directory, path, child, failure)
    }

    pub(crate) fn claim_child(
        parent: &Self,
        child: &str,
        failure: Option<ClaimFailurePoint>,
    ) -> Result<Self, FixtureClaimFailure> {
        let directory = parent.directory.try_clone().map_err(|error| {
            FixtureClaimFailure::Failed(FixtureScopeError::Claim(format!(
                "parent descriptor clone: {error}"
            )))
        })?;
        Self::claim_from_parent(directory, parent.path.join(child), child, failure)
    }

    fn claim_from_parent(
        parent_directory: File,
        path: PathBuf,
        child: &str,
        failure: Option<ClaimFailurePoint>,
    ) -> Result<Self, FixtureClaimFailure> {
        let name = CString::new(child).map_err(|_| {
            FixtureClaimFailure::Failed(FixtureScopeError::Claim(
                "fixture child name contains NUL".to_owned(),
            ))
        })?;
        if unsafe { libc::mkdirat(parent_directory.as_raw_fd(), name.as_ptr(), 0o700) } != 0 {
            return Err(FixtureClaimFailure::Failed(FixtureScopeError::Claim(
                io::Error::last_os_error().to_string(),
            )));
        }
        let identity = match capture_identity(&parent_directory, &name) {
            Ok(value) => value,
            Err(error) => {
                return Err(FixtureClaimFailure::Retained(ClaimResidue::Unopened(
                    UnopenedClaimResidue {
                        parent: parent_directory,
                        name,
                        path,
                        identity: None,
                        error,
                    },
                )));
            }
        };
        if failure == Some(ClaimFailurePoint::AfterMkdirBeforeOpen) {
            return Err(FixtureClaimFailure::Retained(ClaimResidue::Unopened(
                UnopenedClaimResidue {
                    parent: parent_directory,
                    name,
                    path,
                    identity: Some(identity),
                    error: FixtureScopeError::Retained(
                        "injected post-mkdir pre-open failure".to_owned(),
                    ),
                },
            )));
        }
        let descriptor = unsafe {
            libc::openat(
                parent_directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if descriptor < 0 {
            return Err(FixtureClaimFailure::Retained(ClaimResidue::Unopened(
                UnopenedClaimResidue {
                    parent: parent_directory,
                    name,
                    path,
                    identity: Some(identity),
                    error: FixtureScopeError::Retained(format!(
                        "open failed: {}",
                        io::Error::last_os_error()
                    )),
                },
            )));
        }
        let directory = unsafe { File::from_raw_fd(descriptor) };
        let residue = OpenedClaimResidue {
            parent: parent_directory,
            directory,
            name,
            path,
            identity,
            error: FixtureScopeError::Retained("opened claim awaits identity".to_owned()),
        };
        if failure == Some(ClaimFailurePoint::AfterOpenBeforeIdentity) {
            return Err(FixtureClaimFailure::Retained(ClaimResidue::Opened(residue)));
        }
        residue
            .reconcile()
            .map_err(|value| FixtureClaimFailure::Retained(ClaimResidue::Opened(value)))
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn has_name_binding(&self) -> bool {
        self.binding == FixtureScopeBinding::Bound
    }

    pub(crate) fn setup_failure(stage: &'static str) -> FixtureScopeError {
        FixtureScopeError::Setup(stage)
    }

    pub(crate) fn rollback(&mut self) -> Result<(), FixtureScopeError> {
        if self.binding == FixtureScopeBinding::Detached {
            return Err(FixtureScopeError::Retained(
                "scope is retained by descriptor identity without a pathname binding".to_owned(),
            ));
        }
        match crate::catalog_fixture_custody::quarantine_and_remove(
            &self.parent,
            &self.directory,
            &self.name,
            self.device,
            self.inode,
        ) {
            crate::catalog_fixture_custody::CleanupDisposition::Deleted => Ok(()),
            crate::catalog_fixture_custody::CleanupDisposition::Retained { binding, reason } => {
                self.apply_retained_binding(binding);
                Err(FixtureScopeError::Retained(reason))
            }
        }
    }

    pub(crate) fn teardown_after_assertions(&mut self) -> Result<(), FixtureScopeError> {
        self.rollback()
    }

    fn apply_retained_binding(&mut self, binding: crate::catalog_fixture_custody::RetainedBinding) {
        match binding {
            crate::catalog_fixture_custody::RetainedBinding::Named(name) => {
                if let Some(parent) = self.path.parent() {
                    self.path = parent.join(name.to_string_lossy().as_ref());
                    self.name = name;
                } else {
                    self.binding = FixtureScopeBinding::Detached;
                }
            }
            crate::catalog_fixture_custody::RetainedBinding::DescriptorOnly => {
                self.binding = FixtureScopeBinding::Detached;
            }
        }
    }
}

pub(crate) fn populate_catalog_scope(
    path: &Path,
    catalog_bytes: &[u8],
    fail_after: Option<CatalogSetupFailurePoint>,
) -> Result<(), FixtureScopeError> {
    inject(fail_after, CatalogSetupFailurePoint::AfterClaim)?;
    for directory in [
        "config",
        "src",
        "tests",
        "target/routine-syntax",
        "target/routine-verify",
    ] {
        fs::create_dir_all(path.join(directory))
            .map_err(|error| FixtureScopeError::Claim(format!("{directory}: {error}")))?;
    }
    inject(fail_after, CatalogSetupFailurePoint::AfterDirectories)?;
    fs::write(path.join("config/routines.json"), catalog_bytes)
        .map_err(|error| FixtureScopeError::Claim(format!("catalog: {error}")))?;
    inject(fail_after, CatalogSetupFailurePoint::AfterCatalogWrite)?;
    fs::write(path.join("src/input.txt"), b"source input\n")
        .map_err(|error| FixtureScopeError::Claim(format!("source: {error}")))?;
    fs::write(path.join("tests/input.txt"), b"test input\n")
        .map_err(|error| FixtureScopeError::Claim(format!("test: {error}")))?;
    Ok(())
}

fn inject(
    fail_after: Option<CatalogSetupFailurePoint>,
    point: CatalogSetupFailurePoint,
) -> Result<(), FixtureScopeError> {
    if fail_after == Some(point) {
        return Err(ClaimedFixtureScope::setup_failure(match point {
            CatalogSetupFailurePoint::AfterClaim => "claim",
            CatalogSetupFailurePoint::AfterDirectories => "directories",
            CatalogSetupFailurePoint::AfterCatalogWrite => "catalog",
        }));
    }
    Ok(())
}
