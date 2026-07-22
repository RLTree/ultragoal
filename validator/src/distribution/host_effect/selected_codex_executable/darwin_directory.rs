use super::super::super::super::HostEffectLedgerError;
use super::super::super::super::{ledger_io, tampered};
use std::fs::File;
use std::path::{Path, PathBuf};

pub(super) struct DarwinPrivateDirectory {
    pub(super) path: PathBuf,
    file: File,
}

impl DarwinPrivateDirectory {
    pub(super) fn create() -> Result<Self, HostEffectLedgerError> {
        use std::os::unix::fs::PermissionsExt;
        use std::sync::atomic::{AtomicU64, Ordering};

        static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);
        let parent = std::env::temp_dir();
        for _ in 0..32 {
            let suffix = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!(
                "harness-ultragoal-codex-{}-{suffix}",
                std::process::id()
            ));
            match std::fs::create_dir(&path) {
                Ok(()) => {
                    let result: Result<Self, HostEffectLedgerError> = (|| {
                        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))
                            .map_err(|_| ledger_io())?;
                        let metadata = std::fs::symlink_metadata(&path).map_err(|_| ledger_io())?;
                        use std::os::unix::fs::MetadataExt;
                        if !metadata.is_dir()
                            || metadata.mode() & 0o7777 != 0o700
                            || metadata.uid() != unsafe { libc::geteuid() }
                        {
                            return Err(tampered());
                        }
                        let file = File::open(&path).map_err(|_| ledger_io())?;
                        Ok(Self {
                            path: path.clone(),
                            file,
                        })
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
        use std::os::fd::AsRawFd;

        // SAFETY: `file` owns the directory descriptor and this sets the
        // documented Darwin user immutable bit.
        if unsafe { libc::fchflags(self.file.as_raw_fd(), libc::UF_IMMUTABLE) } != 0 {
            return Err(ledger_io());
        }
        Ok(())
    }

    fn clear_immutable(&self) -> Result<(), HostEffectLedgerError> {
        use std::os::fd::AsRawFd;

        // SAFETY: `file` owns the descriptor and this is an explicit finalization.
        if unsafe { libc::fchflags(self.file.as_raw_fd(), 0) } != 0 {
            return Err(ledger_io());
        }
        Ok(())
    }

    pub(super) fn finalize(self, file_path: &Path) -> Result<(), HostEffectLedgerError> {
        self.clear_immutable()?;
        clear_immutable_and_remove(file_path)?;
        match std::fs::remove_dir(&self.path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(ledger_io()),
        }
    }
}

fn clear_immutable_and_remove(path: &Path) -> Result<(), HostEffectLedgerError> {
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(ledger_io()),
    };
    let canonical_parent = path
        .parent()
        .and_then(|parent| std::fs::canonicalize(parent).ok());
    if !metadata.is_file()
        || metadata.nlink() != 1
        || metadata.mode() & 0o7777 != 0o700
        || metadata.uid() != unsafe { libc::geteuid() }
        || std::fs::canonicalize(path)
            .ok()
            .and_then(|canonical| canonical.parent().map(Path::to_owned))
            .as_deref()
            != canonical_parent.as_deref()
    {
        return Err(tampered());
    }
    let mut options = std::fs::OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
    let file = options.open(path).map_err(|_| ledger_io())?;
    let open_metadata = file.metadata().map_err(|_| ledger_io())?;
    if open_metadata.dev() != metadata.dev() || open_metadata.ino() != metadata.ino() {
        return Err(tampered());
    }
    // SAFETY: `file` owns the descriptor and clearing flags enables removal.
    if unsafe { libc::fchflags(file.as_raw_fd(), 0) } != 0 {
        return Err(ledger_io());
    }
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(ledger_io()),
    }
}
