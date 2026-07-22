use super::super::super::super::HostEffectLedgerError;
use super::super::super::super::{ledger_io, tampered};
use std::ffi::CString;
use std::fs::File;
use std::os::darwin::fs::MetadataExt as DarwinMetadataExt;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

pub(super) struct DarwinPrivateDirectory {
    pub(super) path: PathBuf,
    file: File,
    parent: File,
}

impl DarwinPrivateDirectory {
    pub(super) fn create() -> Result<Self, HostEffectLedgerError> {
        use std::os::unix::fs::PermissionsExt;
        use std::sync::atomic::{AtomicU64, Ordering};

        static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);
        let parent_path = std::env::temp_dir();
        let parent = File::open(&parent_path).map_err(|_| ledger_io())?;
        for _ in 0..32 {
            let suffix = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = parent_path.join(format!(
                "harness-ultragoal-codex-{}-{suffix}",
                std::process::id()
            ));
            match std::fs::create_dir(&path) {
                Ok(()) => {
                    let result: Result<Self, HostEffectLedgerError> = (|| {
                        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))
                            .map_err(|_| ledger_io())?;
                        let file = File::open(&path).map_err(|_| ledger_io())?;
                        let directory = Self {
                            path: path.clone(),
                            file,
                            parent: parent.try_clone().map_err(|_| ledger_io())?,
                        };
                        directory.check_identity(false)?;
                        Ok(directory)
                    })();
                    return match result {
                        Ok(directory) => Ok(directory),
                        Err(error) => {
                            let _ = std::fs::remove_dir(&path);
                            Err(error)
                        }
                    };
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(_) => return Err(ledger_io()),
            }
        }
        Err(ledger_io())
    }

    pub(super) fn make_immutable(&self) -> Result<(), HostEffectLedgerError> {
        // SAFETY: `file` owns the directory descriptor and this sets the
        // documented Darwin user immutable bit.
        if unsafe { libc::fchflags(self.file.as_raw_fd(), libc::UF_IMMUTABLE) } != 0 {
            return Err(ledger_io());
        }
        Ok(())
    }

    pub(super) fn revalidate(&self) -> Result<(), HostEffectLedgerError> {
        self.check_identity(true)
    }

    fn check_identity(&self, require_immutable: bool) -> Result<(), HostEffectLedgerError> {
        let descriptor = self.file.metadata().map_err(|_| ledger_io())?;
        let path = std::fs::symlink_metadata(&self.path).map_err(|_| ledger_io())?;
        let parent = self.path.parent().ok_or_else(ledger_io)?;
        let parent_metadata = std::fs::symlink_metadata(parent).map_err(|_| ledger_io())?;
        let parent_descriptor = self.parent.metadata().map_err(|_| ledger_io())?;
        if !path.is_dir()
            || path.mode() & 0o7777 != 0o700
            || path.uid() != unsafe { libc::geteuid() }
            || !same_object(&path, &descriptor)
            || !parent_metadata.is_dir()
            || !same_object(&parent_metadata, &parent_descriptor)
            || (require_immutable && path.st_flags() & libc::UF_IMMUTABLE == 0)
        {
            return Err(tampered());
        }
        Ok(())
    }

    pub(super) fn finalize(
        self,
        retained_file: Option<&File>,
        file_path: &Path,
    ) -> Result<(), HostEffectLedgerError> {
        self.check_identity(false)?;
        let file_name = file_path.file_name().ok_or_else(ledger_io)?;
        if file_path.parent() != Some(self.path.as_path()) {
            return Err(tampered());
        }
        // SAFETY: `file` owns the descriptor and this is an explicit finalization.
        if unsafe { libc::fchflags(self.file.as_raw_fd(), 0) } != 0 {
            return Err(ledger_io());
        }
        if let Some(file) = open_at(self.file.as_raw_fd(), file_name.as_bytes())? {
            let metadata = file.metadata().map_err(|_| ledger_io())?;
            if !metadata.is_file()
                || metadata.mode() & 0o7777 != 0o700
                || metadata.uid() != unsafe { libc::geteuid() }
            {
                return Err(tampered());
            }
            if let Some(retained_file) = retained_file {
                let retained_metadata = retained_file.metadata().map_err(|_| ledger_io())?;
                if !same_object(&retained_metadata, &metadata) {
                    return Err(tampered());
                }
            }
            // SAFETY: `file` owns this descriptor and clearing flags enables removal.
            if unsafe { libc::fchflags(file.as_raw_fd(), 0) } != 0 {
                return Err(ledger_io());
            }
            unlink_at(self.file.as_raw_fd(), file_name.as_bytes(), 0)?;
        } else if retained_file.is_some() {
            return Err(tampered());
        }

        let directory_name = self.path.file_name().ok_or_else(ledger_io)?;
        let current_directory =
            open_at(self.parent.as_raw_fd(), directory_name.as_bytes())?.ok_or_else(tampered)?;
        let retained_directory = self.file.metadata().map_err(|_| ledger_io())?;
        if !same_object(
            &current_directory.metadata().map_err(|_| ledger_io())?,
            &retained_directory,
        ) {
            return Err(tampered());
        }
        unlink_at(
            self.parent.as_raw_fd(),
            directory_name.as_bytes(),
            libc::AT_REMOVEDIR,
        )
    }
}

fn open_at(dirfd: libc::c_int, name: &[u8]) -> Result<Option<File>, HostEffectLedgerError> {
    let name = CString::new(name).map_err(|_| ledger_io())?;
    // SAFETY: `name` is a live NUL-terminated component and `dirfd` is retained.
    let fd = unsafe {
        libc::openat(
            dirfd,
            name.as_ptr(),
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            0,
        )
    };
    if fd < 0 {
        return match std::io::Error::last_os_error().raw_os_error() {
            Some(libc::ENOENT) => Ok(None),
            _ => Err(ledger_io()),
        };
    }
    // SAFETY: `fd` is a fresh descriptor owned by this File.
    Ok(Some(unsafe { File::from_raw_fd(fd) }))
}

fn unlink_at(
    dirfd: libc::c_int,
    name: &[u8],
    flags: libc::c_int,
) -> Result<(), HostEffectLedgerError> {
    let name = CString::new(name).map_err(|_| ledger_io())?;
    // SAFETY: `name` is a live NUL-terminated component and `dirfd` is retained.
    if unsafe { libc::unlinkat(dirfd, name.as_ptr(), flags) } != 0 {
        return Err(ledger_io());
    }
    Ok(())
}

fn same_object(left: &std::fs::Metadata, right: &std::fs::Metadata) -> bool {
    left.dev() == right.dev() && left.ino() == right.ino()
}
