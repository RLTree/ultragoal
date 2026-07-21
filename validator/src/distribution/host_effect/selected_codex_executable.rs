use super::{
    HostEffectLedgerError, HostEffectLedgerErrorId, MAX_PINNED_EXECUTABLE_BYTES, invalid_record,
    ledger_io, read_at_retry, same_executable_object, tampered,
};
use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use sha2::{Digest, Sha256};
use std::env;
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

mod identity;
use identity::SelectedCodexExecutableIdentity;
mod execution;

pub(crate) struct SelectedCodexExecutable {
    file: File,
    identity: SelectedCodexExecutableIdentity,
}

impl SelectedCodexExecutable {
    fn pin(path: &Path) -> Result<Self, HostEffectLedgerError> {
        #[cfg(unix)]
        {
            let canonical = fs::canonicalize(path).map_err(|_| ledger_io())?;
            if !canonical.is_absolute() {
                return Err(invalid_record());
            }
            let mut options = OpenOptions::new();
            options
                .read(true)
                .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK);
            let file = options.open(&canonical).map_err(|_| ledger_io())?;
            let identity = capture_executable_identity(&file, &canonical)?;
            Ok(Self { file, identity })
        }
        #[cfg(not(unix))]
        {
            let _ = path;
            Err(invalid_record())
        }
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    fn raw_file(&self) -> &File {
        &self.file
    }

    #[cfg(target_os = "macos")]
    fn raw_loaded_identity(&self) -> (&Path, u64, u64) {
        (
            Path::new(&self.identity.canonical_path),
            self.identity.device,
            self.identity.inode,
        )
    }

    pub(in crate::distribution::host_effect) fn binding_sha256(
        &self,
    ) -> Result<String, HostEffectLedgerError> {
        self.identity.binding_sha256()
    }

    pub(in crate::distribution::host_effect) fn duplicate(
        &self,
    ) -> Result<Self, HostEffectLedgerError> {
        Ok(Self {
            file: self.file.try_clone().map_err(|_| ledger_io())?,
            identity: self.identity.clone(),
        })
    }

    pub(in crate::distribution::host_effect) fn revalidate(
        &self,
    ) -> Result<(), HostEffectLedgerError> {
        #[cfg(unix)]
        {
            let current =
                capture_executable_identity(&self.file, Path::new(&self.identity.canonical_path))?;
            if current != self.identity {
                return Err(HostEffectLedgerError::new(
                    HostEffectLedgerErrorId::Tampered,
                ));
            }
            Ok(())
        }
        #[cfg(not(unix))]
        {
            Err(invalid_record())
        }
    }

    pub(in crate::distribution::host_effect) fn execute(
        &self,
        capability: &super::lifecycle::DescriptorExecutionCapability,
        command: &super::super::HostCommand,
        policy: &super::executor::HostEffectExecutionPolicy,
        cancellation: &super::executor::HostEffectCancellation,
        cwd: std::os::fd::RawFd,
    ) -> Result<super::executor::CommandCapture, super::executor::BackendFailure> {
        execution::execute(self, capability, command, policy, cancellation, cwd)
    }
}

pub(crate) fn resolve_codex_executable() -> Result<SelectedCodexExecutable, DistributionError> {
    let path = env::var_os("PATH").ok_or_else(|| error(DistributionErrorId::ObjectUnavailable))?;
    if path_byte_length(&path) > MAX_PATH_BYTES {
        return Err(error(DistributionErrorId::ObjectTooLarge));
    }
    resolve_from_path(env::split_paths(&path))
}

const CODEX_PROGRAM: &str = "codex";
const MAX_PATH_BYTES: usize = 64 * 1024;

fn path_byte_length(path: &OsStr) -> usize {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;

        path.as_bytes().len()
    }
    #[cfg(not(unix))]
    {
        path.to_string_lossy().len()
    }
}

fn resolve_from_path(
    paths: impl IntoIterator<Item = PathBuf>,
) -> Result<SelectedCodexExecutable, DistributionError> {
    for directory in paths {
        if !directory.is_absolute() {
            continue;
        }
        let candidate = directory.join(CODEX_PROGRAM);
        if let Ok(executable) = SelectedCodexExecutable::pin(&candidate) {
            return Ok(executable);
        }
    }
    Err(error(DistributionErrorId::ObjectUnavailable))
}

#[cfg(unix)]
fn capture_executable_identity(
    file: &File,
    canonical_path: &Path,
) -> Result<SelectedCodexExecutableIdentity, HostEffectLedgerError> {
    let path_before = fs::symlink_metadata(canonical_path).map_err(|_| ledger_io())?;
    let descriptor_before = file.metadata().map_err(|_| ledger_io())?;
    validate_executable_metadata(&path_before)?;
    validate_executable_metadata(&descriptor_before)?;
    if !same_executable_object(&path_before, &descriptor_before) {
        return Err(tampered());
    }

    let mut hasher = Sha256::new();
    let mut offset = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    while offset < descriptor_before.len() {
        let remaining = (descriptor_before.len() - offset).min(buffer.len() as u64) as usize;
        let count = read_at_retry(file, &mut buffer[..remaining], offset)?;
        if count == 0 {
            return Err(tampered());
        }
        hasher.update(&buffer[..count]);
        offset = offset
            .checked_add(count as u64)
            .ok_or_else(invalid_record)?;
    }
    let mut extra = [0_u8; 1];
    if read_at_retry(file, &mut extra, descriptor_before.len())? != 0 {
        return Err(tampered());
    }

    let descriptor_after = file.metadata().map_err(|_| ledger_io())?;
    let path_after = fs::symlink_metadata(canonical_path).map_err(|_| ledger_io())?;
    validate_executable_metadata(&descriptor_after)?;
    validate_executable_metadata(&path_after)?;
    if !same_executable_object(&descriptor_before, &descriptor_after)
        || !same_executable_object(&descriptor_after, &path_after)
        || fs::canonicalize(canonical_path).map_err(|_| ledger_io())? != canonical_path
    {
        return Err(tampered());
    }
    let canonical_path = canonical_path
        .to_str()
        .ok_or_else(invalid_record)?
        .to_owned();
    Ok(SelectedCodexExecutableIdentity {
        canonical_path,
        content_sha256: format!("sha256:{:x}", hasher.finalize()),
        device: descriptor_after.dev(),
        inode: descriptor_after.ino(),
        mode: descriptor_after.mode(),
        uid: descriptor_after.uid(),
        gid: descriptor_after.gid(),
        hard_links: descriptor_after.nlink(),
        size: descriptor_after.len(),
        modified_seconds: descriptor_after.mtime(),
        modified_nanoseconds: descriptor_after.mtime_nsec(),
        changed_seconds: descriptor_after.ctime(),
        changed_nanoseconds: descriptor_after.ctime_nsec(),
    })
}

#[cfg(unix)]
fn validate_executable_metadata(metadata: &fs::Metadata) -> Result<(), HostEffectLedgerError> {
    if !metadata.file_type().is_file()
        || metadata.len() == 0
        || metadata.len() > MAX_PINNED_EXECUTABLE_BYTES
        || metadata.nlink() != 1
        || metadata.mode() & 0o111 == 0
    {
        return Err(invalid_record());
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod selected_tests;

#[cfg(all(test, unix))]
pub(crate) use selected_tests::{SelectedCodexExecutableTestFixture, test_fixture};
