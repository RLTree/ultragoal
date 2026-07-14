#[cfg(unix)]
use super::owned::OwnedFile;
use super::root::ConfinedRoot;
use crate::distribution::cache::MarketplaceEffects;
use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::install::{ExpectedPrior, InstallEffects, InstalledPostimage};
use crate::distribution::package::PackageEffects;
use crate::distribution::reader::sha256;
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(unix)]
use super::descriptor::read_file;
#[cfg(unix)]
use super::file_transition::{same_snapshot, transition};

pub(super) const FILE_LIMIT: usize = 65 * 1024 * 1024;
static NONCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug)]
pub struct ScopedFile {
    root: ConfinedRoot,
    relative: String,
}

impl ScopedFile {
    pub fn new(root: ConfinedRoot, relative: &str) -> Result<Self, DistributionError> {
        root.validate_relative(relative)?;
        Ok(Self {
            root,
            relative: relative.to_owned(),
        })
    }

    pub(crate) fn root_id(&self) -> &str {
        self.root.root_id()
    }

    pub(crate) fn relative_path(&self) -> &str {
        &self.relative
    }

    #[cfg(unix)]
    pub fn inspect(&self, maximum: usize) -> Result<Option<Vec<u8>>, DistributionError> {
        Ok(self
            .inspect_snapshot(maximum)?
            .map(|snapshot| snapshot.bytes))
    }

    #[cfg(unix)]
    pub(crate) fn installed_postimage(
        &self,
        maximum: usize,
    ) -> Result<Option<InstalledPostimage>, DistributionError> {
        let Some(snapshot) = self.inspect_snapshot(maximum)? else {
            return Ok(None);
        };
        Ok(Some(InstalledPostimage::new(
            self.root_id().into(),
            self.relative_path(),
            sha256(&snapshot.bytes),
            sha256(
                format!(
                    "{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}",
                    snapshot.identity.device,
                    snapshot.identity.inode,
                    snapshot.identity.length,
                    snapshot.identity.modified_seconds,
                    snapshot.identity.modified_nanos,
                    snapshot.identity.changed_seconds,
                    snapshot.identity.changed_nanos,
                    snapshot.mode
                )
                .as_bytes(),
            ),
            snapshot.identity.length,
            snapshot.mode,
        )))
    }

    #[cfg(unix)]
    fn inspect_snapshot(
        &self,
        maximum: usize,
    ) -> Result<Option<super::descriptor::FileSnapshot>, DistributionError> {
        let (parent, name) = match self.root.parent(&self.relative, false) {
            Ok(value) => value,
            Err(failure) if failure.id() == DistributionErrorId::ObjectUnavailable => {
                return Ok(None);
            }
            Err(failure) => return Err(failure),
        };
        let snapshot = read_file(&parent, &name, maximum)?;
        self.root.revalidate_parent(&self.relative, &parent)?;
        Ok(snapshot)
    }

    #[cfg(not(unix))]
    pub fn inspect(&self, _maximum: usize) -> Result<Option<Vec<u8>>, DistributionError> {
        Err(error(DistributionErrorId::CapabilityMismatch))
    }

    #[cfg(not(unix))]
    pub(crate) fn installed_postimage(
        &self,
        _maximum: usize,
    ) -> Result<Option<InstalledPostimage>, DistributionError> {
        Err(error(DistributionErrorId::CapabilityMismatch))
    }

    #[cfg(unix)]
    pub fn apply(
        &self,
        expected_sha256: Option<&str>,
        replacement: Option<&[u8]>,
    ) -> Result<bool, DistributionError> {
        if replacement.is_some_and(|row| row.len() > FILE_LIMIT) {
            return Err(error(DistributionErrorId::ObjectTooLarge));
        }
        let root = self.root.root_directory()?;
        let lock_name = format!(".hul-lock-{}", &sha256(self.relative.as_bytes())[7..]);
        let _lock = OwnedFile::create(root.duplicate()?, lock_name, &[], 0o600, true)?;
        let (parent, name) = self.root.parent(&self.relative, true)?;
        let before = read_file(&parent, &name, FILE_LIMIT)?;
        if before.as_ref().map(|row| sha256(&row.bytes)).as_deref() != expected_sha256 {
            return Ok(false);
        }
        let stage_name = format!(
            ".hul-stage-{}-{}-{}",
            std::process::id(),
            NONCE.fetch_add(1, Ordering::Relaxed),
            &sha256(self.relative.as_bytes())[7..],
        );
        let mut stage = replacement
            .map(|bytes| {
                OwnedFile::create(root.duplicate()?, stage_name.clone(), bytes, 0o600, false)
            })
            .transpose()?;
        let current = read_file(&parent, &name, FILE_LIMIT)?;
        if !same_snapshot(current.as_ref(), before.as_ref()) {
            return Ok(false);
        }
        self.root.revalidate_parent(&self.relative, &parent)?;
        let transitioned = transition(
            &root,
            &parent,
            &name,
            &stage_name,
            before.as_ref(),
            replacement,
            stage.as_mut(),
        )?;
        if !transitioned {
            return Ok(false);
        }
        self.root.revalidate_parent(&self.relative, &parent)?;
        let after = read_file(&parent, &name, FILE_LIMIT)?;
        if after.as_ref().map(|row| row.bytes.as_slice()) != replacement {
            return Err(error(DistributionErrorId::ObjectChanged));
        }
        Ok(true)
    }

    #[cfg(not(unix))]
    pub fn apply(
        &self,
        _expected_sha256: Option<&str>,
        _replacement: Option<&[u8]>,
    ) -> Result<bool, DistributionError> {
        Err(error(DistributionErrorId::CapabilityMismatch))
    }
}

impl PackageEffects for ScopedFile {
    fn read_package(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.inspect(maximum).map_err(|_| ())
    }

    fn compare_exchange_package(
        &mut self,
        expected_sha256: Option<&str>,
        replacement: Option<&[u8]>,
    ) -> Result<bool, ()> {
        self.apply(expected_sha256, replacement).map_err(|_| ())
    }
}

impl MarketplaceEffects for ScopedFile {
    fn read(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.inspect(maximum).map_err(|_| ())
    }

    fn compare_exchange(
        &mut self,
        expected_sha256: Option<&str>,
        replacement: Option<&[u8]>,
    ) -> Result<bool, ()> {
        self.apply(expected_sha256, replacement).map_err(|_| ())
    }
}

#[derive(Clone, Debug)]
pub struct ScopedInstall {
    root: ConfinedRoot,
}

impl ScopedInstall {
    pub fn new(root: ConfinedRoot) -> Self {
        Self { root }
    }
}

impl InstallEffects for ScopedInstall {
    fn read_installed(&mut self, target: &str, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        ScopedFile::new(self.root.clone(), target)
            .and_then(|row| row.inspect(maximum))
            .map_err(|_| ())
    }

    fn installed_postimage(
        &mut self,
        target: &str,
        maximum: usize,
    ) -> Result<Option<InstalledPostimage>, ()> {
        ScopedFile::new(self.root.clone(), target)
            .and_then(|row| row.installed_postimage(maximum))
            .map_err(|_| ())
    }

    fn compare_exchange_installed(
        &mut self,
        target: &str,
        expected: &ExpectedPrior,
        replacement: Option<&[u8]>,
    ) -> Result<bool, ()> {
        let expected = match expected {
            ExpectedPrior::Absent => None,
            ExpectedPrior::ExactDigest(value) => Some(value.as_str()),
        };
        ScopedFile::new(self.root.clone(), target)
            .and_then(|row| row.apply(expected, replacement))
            .map_err(|_| ())
    }
}
