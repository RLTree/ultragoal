use super::super::HostEffectLedgerError;
#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "macos"))]
use super::super::ledger_io;
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
use super::super::tampered;
use std::fs::File;

#[cfg(target_os = "macos")]
#[path = "darwin_copy.rs"]
mod darwin_copy;

/// The only executable object accepted by a descriptor launch backend.
pub(super) struct ImmutableExecutable {
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    file: File,
    #[cfg(target_os = "macos")]
    darwin: std::sync::Arc<darwin_copy::DarwinPrivateExecutable>,
}

impl ImmutableExecutable {
    pub(super) fn stage(
        source: &File,
        mode: u32,
        expected_size: u64,
        expected_sha256: &str,
    ) -> Result<Self, HostEffectLedgerError> {
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        {
            use sha2::{Digest, Sha256};
            use std::ffi::CString;
            use std::io::Write;
            use std::os::fd::{AsRawFd, FromRawFd};

            let name = CString::new("harness-ultragoal-codex").map_err(|_| ledger_io())?;
            // SAFETY: the C string remains live for the syscall and the returned
            // descriptor is owned by the File constructed below.
            let descriptor = unsafe {
                libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING)
            };
            if descriptor < 0 {
                return Err(ledger_io());
            }
            // SAFETY: memfd_create returned a unique owned descriptor.
            let file = unsafe { File::from_raw_fd(descriptor) };
            // SAFETY: the descriptor is owned by `file` and mode came from the
            // already validated regular executable.
            if unsafe { libc::fchmod(file.as_raw_fd(), (mode & 0o7777) as libc::mode_t) } != 0 {
                return Err(ledger_io());
            }
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
                file.write_all(&buffer[..count]).map_err(|_| ledger_io())?;
                offset = offset.checked_add(count as u64).ok_or_else(ledger_io)?;
            }
            if offset != expected_size
                || format!("sha256:{:x}", hasher.finalize()) != expected_sha256
            {
                return Err(tampered());
            }
            let required =
                libc::F_SEAL_SEAL | libc::F_SEAL_SHRINK | libc::F_SEAL_GROW | libc::F_SEAL_WRITE;
            // SAFETY: the descriptor is owned by `file` and the command is the
            // documented memfd seal operation.
            if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_ADD_SEALS, required) } < 0 {
                return Err(ledger_io());
            }
            // SAFETY: GET_SEALS writes no memory and returns the seal mask.
            let seals = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GET_SEALS) };
            if seals < 0 || seals & required != required {
                return Err(tampered());
            }
            Ok(Self { file })
        }
        #[cfg(target_os = "macos")]
        {
            Ok(Self {
                darwin: std::sync::Arc::new(darwin_copy::DarwinPrivateExecutable::stage(
                    source,
                    mode,
                    expected_size,
                    expected_sha256,
                )?),
            })
        }
        #[cfg(not(any(target_os = "linux", target_os = "freebsd", target_os = "macos")))]
        {
            let _ = (source, mode, expected_size, expected_sha256);
            Err(ledger_io())
        }
    }

    #[cfg(test)]
    pub(super) fn duplicate(&self) -> Result<Self, HostEffectLedgerError> {
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        {
            return Ok(Self {
                file: self.file.try_clone().map_err(|_| ledger_io())?,
            });
        }
        #[cfg(target_os = "macos")]
        {
            Ok(Self {
                darwin: std::sync::Arc::clone(&self.darwin),
            })
        }
        #[cfg(not(any(target_os = "linux", target_os = "freebsd", target_os = "macos")))]
        {
            Err(ledger_io())
        }
    }

    pub(super) fn finalize(self) -> Result<(), HostEffectLedgerError> {
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        {
            drop(self);
            return Ok(());
        }
        #[cfg(target_os = "macos")]
        {
            return std::sync::Arc::try_unwrap(self.darwin)
                .map_err(|_| ledger_io())?
                .finalize();
        }
        #[cfg(not(any(target_os = "linux", target_os = "freebsd", target_os = "macos")))]
        {
            let _ = self;
            Err(ledger_io())
        }
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    pub(super) fn file(&self) -> &File {
        &self.file
    }

    #[cfg(target_os = "macos")]
    pub(super) fn path(&self) -> &std::path::Path {
        self.darwin.path()
    }

    #[cfg(target_os = "macos")]
    pub(super) fn revalidate(&self) -> Result<(), HostEffectLedgerError> {
        self.darwin.revalidate()
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "macos"))]
fn read_at_retry(
    file: &File,
    buffer: &mut [u8],
    offset: u64,
) -> Result<usize, HostEffectLedgerError> {
    use std::os::unix::fs::FileExt;

    loop {
        match file.read_at(buffer, offset) {
            Ok(count) => return Ok(count),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return Err(ledger_io()),
        }
    }
}
