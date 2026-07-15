use std::ffi::CString;
use std::fs::{self, File};
use std::io;
use std::os::fd::{AsRawFd, FromRawFd};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum FixtureScopeError {
    Claim(String),
    Setup(&'static str),
    Substituted,
    Rollback(String),
    Retained(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CatalogSetupFailurePoint {
    AfterClaim,
    AfterDirectories,
    AfterCatalogWrite,
}

pub(crate) struct ClaimedFixtureScope {
    path: PathBuf,
    parent: File,
    directory: File,
    name: CString,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
}

impl ClaimedFixtureScope {
    pub(crate) fn claim(parent: &Path, child: &str) -> Result<Self, FixtureScopeError> {
        fs::create_dir_all(parent)
            .map_err(|error| FixtureScopeError::Claim(format!("parent: {error}")))?;
        let parent_directory = File::open(parent)
            .map_err(|error| FixtureScopeError::Claim(format!("parent open: {error}")))?;
        let path = parent.join(child);
        let name = CString::new(child)
            .map_err(|_| FixtureScopeError::Claim("fixture child name contains NUL".to_owned()))?;
        if unsafe { libc::mkdirat(parent_directory.as_raw_fd(), name.as_ptr(), 0o700) } != 0 {
            return Err(FixtureScopeError::Claim(
                io::Error::last_os_error().to_string(),
            ));
        }
        let descriptor = unsafe {
            libc::openat(
                parent_directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if descriptor < 0 {
            return Err(FixtureScopeError::Claim(format!(
                "claimed child retained after open failure: {}",
                io::Error::last_os_error()
            )));
        }
        let directory = unsafe { File::from_raw_fd(descriptor) };
        let metadata = directory
            .metadata()
            .map_err(|error| FixtureScopeError::Claim(format!("identity: {error}")))?;
        if !metadata.file_type().is_dir() {
            return Err(FixtureScopeError::Claim(
                "claimed child is not a directory".to_owned(),
            ));
        }
        Ok(Self {
            path,
            parent: parent_directory,
            directory,
            name,
            #[cfg(unix)]
            device: metadata.dev(),
            #[cfg(unix)]
            inode: metadata.ino(),
        })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn setup_failure(stage: &'static str) -> FixtureScopeError {
        FixtureScopeError::Setup(stage)
    }

    pub(crate) fn rollback(&mut self) -> Result<(), FixtureScopeError> {
        match crate::catalog_fixture_custody::quarantine_and_remove(
            &self.parent,
            &self.directory,
            &self.name,
            self.device,
            self.inode,
        ) {
            crate::catalog_fixture_custody::CleanupDisposition::Deleted => Ok(()),
            crate::catalog_fixture_custody::CleanupDisposition::Retained { name, reason } => {
                self.path = self
                    .path
                    .parent()
                    .expect("fixture path has parent")
                    .join(name.to_string_lossy().as_ref());
                self.name = name;
                Err(FixtureScopeError::Retained(reason))
            }
        }
    }

    pub(crate) fn teardown_after_assertions(&mut self) -> Result<(), FixtureScopeError> {
        self.rollback()
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
