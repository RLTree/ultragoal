use super::super::HostEffectLedgerError;
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
use super::super::{ledger_io, tampered};
use std::fs::File;

/// The only executable object accepted by the descriptor launch backends.
///
/// On Linux and FreeBSD this is a sealed, private-memory copy of the bytes
/// selected from PATH. Darwin deliberately carries an unsupported marker: the
/// host has no equivalent byte-sealing primitive in this candidate, so its
/// execution backend refuses before any spawn.
pub(super) struct ImmutableExecutable {
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    file: File,
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
            use std::os::unix::fs::FileExt;

            let name = CString::new("harness-ultragoal-codex").map_err(|_| ledger_io())?;
            // SAFETY: the C string is NUL-terminated and remains live for the
            // duration of the syscall; the returned descriptor is owned below.
            let descriptor = unsafe {
                libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING)
            };
            if descriptor < 0 {
                return Err(ledger_io());
            }
            // SAFETY: memfd_create returned a unique owned descriptor.
            let file = unsafe { File::from_raw_fd(descriptor) };
            // SAFETY: the descriptor is owned by `file` and mode is copied
            // from the already validated regular executable.
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
            // SAFETY: the descriptor is owned by `file`; the command and seal
            // mask are the documented fcntl ABI values.
            if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_ADD_SEALS, required) } < 0 {
                return Err(ledger_io());
            }
            // SAFETY: the descriptor is owned by `file` and GET_SEALS writes
            // no memory; it returns the immutable seal mask.
            let seals = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GET_SEALS) };
            if seals < 0 || seals & required != required {
                return Err(tampered());
            }
            Ok(Self { file })
        }
        #[cfg(target_os = "macos")]
        {
            let _ = (source, mode, expected_size, expected_sha256);
            // Darwin has no byte-sealing launch primitive here. Selection
            // remains inspectable for identity and authorization tests, but
            // its execution backend is explicitly unsupported.
            Ok(Self {})
        }
        #[cfg(not(any(target_os = "linux", target_os = "freebsd", target_os = "macos")))]
        {
            let _ = (source, mode, expected_size, expected_sha256);
            Err(ledger_io())
        }
    }

    pub(super) fn duplicate(&self) -> Result<Self, HostEffectLedgerError> {
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        {
            return Ok(Self {
                file: self.file.try_clone().map_err(|_| ledger_io())?,
            });
        }
        #[cfg(target_os = "macos")]
        {
            Ok(Self {})
        }
        #[cfg(not(any(target_os = "linux", target_os = "freebsd", target_os = "macos")))]
        {
            Err(ledger_io())
        }
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    pub(super) fn file(&self) -> &File {
        &self.file
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
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
