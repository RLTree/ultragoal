use super::super::{
    HostEffectLedgerError, HostEffectLedgerErrorId, MAX_PINNED_EXECUTABLE_BYTES, invalid_record,
    ledger_io, read_at_retry, same_executable_object, tampered,
};
use super::identity::SelectedCodexExecutableIdentity;
use super::immutable_launch::ImmutableExecutable;
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

pub(crate) struct SelectedCodexExecutable {
    file: File,
    launch: ImmutableExecutable,
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
            let identity = capture_identity(&file, &canonical)?;
            let launch = ImmutableExecutable::stage(
                &file,
                identity.mode,
                identity.size,
                &identity.content_sha256,
            )?;
            Ok(Self {
                file,
                launch,
                identity,
            })
        }
        #[cfg(not(unix))]
        {
            let _ = path;
            Err(invalid_record())
        }
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    pub(super) fn launch_file(&self) -> &File {
        self.launch.file()
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
            launch: self.launch.duplicate()?,
            identity: self.identity.clone(),
        })
    }

    pub(in crate::distribution::host_effect) fn revalidate(
        &self,
    ) -> Result<(), HostEffectLedgerError> {
        #[cfg(unix)]
        {
            let current = capture_identity(&self.file, Path::new(&self.identity.canonical_path))?;
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
        capability: &super::super::lifecycle::DescriptorExecutionCapability,
        command: &super::super::super::HostCommand,
        policy: &super::super::executor::HostEffectExecutionPolicy,
        cancellation: &super::super::executor::HostEffectCancellation,
        cwd: std::os::fd::RawFd,
    ) -> Result<super::super::executor::CommandCapture, super::super::executor::BackendFailure>
    {
        self.revalidate().map_err(|_| {
            super::super::executor::BackendFailure::before_start(
                super::super::executor::HostEffectExecutorErrorId::ExecutableMutation,
            )
        })?;
        super::execution::execute(self, capability, command, policy, cancellation, cwd)
    }
}

pub(super) fn resolve_from_path(
    paths: impl IntoIterator<Item = PathBuf>,
    program: &str,
) -> Result<SelectedCodexExecutable, HostEffectLedgerError> {
    for directory in paths {
        if !directory.is_absolute() {
            continue;
        }
        let candidate = directory.join(program);
        if let Ok(executable) = SelectedCodexExecutable::pin(&candidate) {
            return Ok(executable);
        }
    }
    Err(ledger_io())
}

#[cfg(unix)]
fn capture_identity(
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
