use self::darwin_directory::DarwinPrivateDirectory;
use super::super::super::{HostEffectLedgerError, ledger_io, tampered};
use super::read_at_retry;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::path::{Path, PathBuf};

#[path = "darwin_directory.rs"]
mod darwin_directory;

pub(super) struct DarwinPrivateExecutable {
    directory: DarwinPrivateDirectory,
    path: PathBuf,
    file: File,
    size: u64,
    sha256: String,
}

impl DarwinPrivateExecutable {
    pub(super) fn stage(
        source: &File,
        mode: u32,
        expected_size: u64,
        expected_sha256: &str,
    ) -> Result<Self, HostEffectLedgerError> {
        use std::io::Write;
        use std::os::fd::AsRawFd;
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

        let _ = mode;
        let directory = DarwinPrivateDirectory::create()?;
        let path = directory.path.join("codex");
        let mut writer = std::fs::OpenOptions::new();
        writer
            .write(true)
            .create_new(true)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
            .mode(0o700);
        let mut writer = writer.open(&path).map_err(|_| ledger_io())?;
        let mut hasher = Sha256::new();
        let mut offset = 0_u64;
        let mut buffer = [0_u8; 64 * 1024];
        while offset < expected_size {
            let remaining = (expected_size - offset).min(buffer.len() as u64) as usize;
            let count = read_at_retry(source, &mut buffer[..remaining], offset)?;
            if count == 0 {
                return Err(tampered());
            }
            hasher.update(&buffer[..count]);
            writer
                .write_all(&buffer[..count])
                .map_err(|_| ledger_io())?;
            offset = offset.checked_add(count as u64).ok_or_else(ledger_io)?;
        }
        if offset != expected_size || format!("sha256:{:x}", hasher.finalize()) != expected_sha256 {
            return Err(tampered());
        }
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| ledger_io())?;
        writer.sync_all().map_err(|_| ledger_io())?;
        // SAFETY: `writer` owns a regular file descriptor and this sets the
        // documented Darwin user immutable bit.
        if unsafe { libc::fchflags(writer.as_raw_fd(), libc::UF_IMMUTABLE) } != 0 {
            return Err(ledger_io());
        }
        drop(writer);
        directory.make_immutable()?;
        let mut options = std::fs::OpenOptions::new();
        options
            .read(true)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK);
        let file = options.open(&path).map_err(|_| ledger_io())?;
        let executable = Self {
            directory,
            path,
            file,
            size: expected_size,
            sha256: expected_sha256.to_owned(),
        };
        executable.revalidate()?;
        Ok(executable)
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn revalidate(&self) -> Result<(), HostEffectLedgerError> {
        use std::os::darwin::fs::MetadataExt as DarwinMetadataExt;
        use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

        let directory = std::fs::symlink_metadata(&self.directory.path).map_err(|_| ledger_io())?;
        let canonical_directory =
            std::fs::canonicalize(&self.directory.path).map_err(|_| ledger_io())?;
        if !directory.is_dir()
            || directory.mode() & 0o7777 != 0o700
            || directory.uid() != unsafe { libc::geteuid() }
            || directory.st_flags() & libc::UF_IMMUTABLE == 0
        {
            return Err(tampered());
        }
        let metadata = std::fs::symlink_metadata(&self.path).map_err(|_| ledger_io())?;
        if !metadata.is_file()
            || metadata.nlink() != 1
            || metadata.mode() & 0o7777 != 0o700
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.st_flags() & libc::UF_IMMUTABLE == 0
            || metadata.len() != self.size
            || std::fs::canonicalize(&self.path)
                .map_err(|_| ledger_io())?
                .parent()
                != Some(canonical_directory.as_path())
        {
            return Err(tampered());
        }
        let descriptor_metadata = self.file.metadata().map_err(|_| ledger_io())?;
        if !descriptor_metadata.is_file()
            || descriptor_metadata.nlink() != 1
            || descriptor_metadata.mode() & 0o7777 != 0o700
            || descriptor_metadata.uid() != unsafe { libc::geteuid() }
            || descriptor_metadata.st_flags() & libc::UF_IMMUTABLE == 0
            || descriptor_metadata.len() != self.size
        {
            return Err(tampered());
        }
        let mut options = std::fs::OpenOptions::new();
        options
            .read(true)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK);
        let path_file = options.open(&self.path).map_err(|_| tampered())?;
        let path_metadata = path_file.metadata().map_err(|_| tampered())?;
        if !path_metadata.is_file()
            || path_metadata.len() != self.size
            || path_metadata.mode() & 0o7777 != 0o700
            || path_metadata.uid() != unsafe { libc::geteuid() }
            || path_metadata.st_flags() & libc::UF_IMMUTABLE == 0
            || path_metadata.dev() != descriptor_metadata.dev()
            || path_metadata.ino() != descriptor_metadata.ino()
        {
            return Err(tampered());
        }
        verify_digest(&self.file, self.size, &self.sha256)?;
        verify_digest(&path_file, self.size, &self.sha256)
    }
}

impl Drop for DarwinPrivateExecutable {
    fn drop(&mut self) {
        self.directory.clear_immutable_for_drop();
        self.directory.remove_file_for_drop(&self.path);
    }
}

fn verify_digest(file: &File, size: u64, expected: &str) -> Result<(), HostEffectLedgerError> {
    let mut hasher = Sha256::new();
    let mut offset = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    while offset < size {
        let remaining = (size - offset).min(buffer.len() as u64) as usize;
        let count = read_at_retry(file, &mut buffer[..remaining], offset)?;
        if count == 0 {
            return Err(tampered());
        }
        hasher.update(&buffer[..count]);
        offset = offset.checked_add(count as u64).ok_or_else(ledger_io)?;
    }
    if offset != size || format!("sha256:{:x}", hasher.finalize()) != expected {
        return Err(tampered());
    }
    Ok(())
}
