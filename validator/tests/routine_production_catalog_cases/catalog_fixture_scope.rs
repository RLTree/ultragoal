use std::fs;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum FixtureScopeError {
    Claim(String),
    Setup(&'static str),
    Substituted,
    Rollback(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CatalogSetupFailurePoint {
    AfterClaim,
    AfterDirectories,
    AfterCatalogWrite,
}

pub(crate) struct ClaimedFixtureScope {
    path: PathBuf,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
}

impl ClaimedFixtureScope {
    pub(crate) fn claim(parent: &Path, child: &str) -> Result<Self, FixtureScopeError> {
        fs::create_dir_all(parent)
            .map_err(|error| FixtureScopeError::Claim(format!("parent: {error}")))?;
        let path = parent.join(child);
        fs::create_dir(&path).map_err(|error| FixtureScopeError::Claim(error.to_string()))?;
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| FixtureScopeError::Claim(format!("identity: {error}")))?;
        if !metadata.file_type().is_dir() {
            return Err(FixtureScopeError::Claim(
                "claimed child is not a directory".to_owned(),
            ));
        }
        Ok(Self {
            path,
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
        self.require_identity()?;
        fs::remove_dir_all(&self.path)
            .map_err(|error| FixtureScopeError::Rollback(error.to_string()))?;
        if self.path.exists() {
            return Err(FixtureScopeError::Rollback(
                "claimed child remains after rollback".to_owned(),
            ));
        }
        Ok(())
    }

    pub(crate) fn release(self) -> PathBuf {
        self.path
    }

    fn require_identity(&self) -> Result<(), FixtureScopeError> {
        let metadata =
            fs::symlink_metadata(&self.path).map_err(|_| FixtureScopeError::Substituted)?;
        if !metadata.file_type().is_dir() {
            return Err(FixtureScopeError::Substituted);
        }
        #[cfg(unix)]
        if metadata.dev() != self.device || metadata.ino() != self.inode {
            return Err(FixtureScopeError::Substituted);
        }
        Ok(())
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
