use super::ClosureError;
use std::ffi::CString;
use std::fs::{self, File, OpenOptions};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Component, Path, PathBuf};

type RootIdentity = (u64, u64, u32);

pub(in crate::plugin_product::source_closure) struct CheckedRoot {
    path: PathBuf,
    file: File,
    identity: RootIdentity,
}

impl CheckedRoot {
    pub(in crate::plugin_product::source_closure) fn path(&self) -> &Path {
        &self.path
    }
}

pub(in crate::plugin_product::source_closure) fn checked_root(
    root: &Path,
) -> Result<CheckedRoot, ClosureError> {
    #[cfg(not(unix))]
    {
        let _ = root;
        return Err(ClosureError::Unreadable);
    }
    #[cfg(unix)]
    {
        let before = fs::symlink_metadata(root).map_err(|_| ClosureError::Missing)?;
        if before.file_type().is_symlink() || !before.is_dir() {
            return Err(ClosureError::SpecialFile);
        }
        let path = fs::canonicalize(root).map_err(|_| ClosureError::Unreadable)?;
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&path)
            .map_err(|_| ClosureError::Unreadable)?;
        let opened = file.metadata().map_err(|_| ClosureError::Unreadable)?;
        let current = fs::symlink_metadata(&path).map_err(|_| ClosureError::Unreadable)?;
        let identity = root_identity(&before);
        if identity != root_identity(&opened)
            || identity != root_identity(&current)
            || !opened.is_dir()
            || current.file_type().is_symlink()
        {
            return Err(ClosureError::FinalSessionDrift);
        }
        Ok(CheckedRoot {
            path,
            file,
            identity,
        })
    }
}

pub(super) fn open_confined(root: &CheckedRoot, relative: &Path) -> Result<File, ClosureError> {
    #[cfg(not(unix))]
    {
        let _ = (root, relative);
        return Err(ClosureError::Unreadable);
    }
    #[cfg(unix)]
    {
        if root_identity(&root.file.metadata().map_err(|_| ClosureError::Unreadable)?)
            != root.identity
        {
            return Err(ClosureError::FinalSessionDrift);
        }
        let mut directory = root
            .file
            .try_clone()
            .map_err(|_| ClosureError::Unreadable)?;
        let components = relative.components().collect::<Vec<_>>();
        for component in &components[..components.len().saturating_sub(1)] {
            let Component::Normal(name) = component else {
                return Err(ClosureError::InvalidPath);
            };
            directory = open_at(
                &directory,
                name.as_bytes(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )?;
        }
        let Some(Component::Normal(name)) = components.last() else {
            return Err(ClosureError::InvalidPath);
        };
        open_at(
            &directory,
            name.as_bytes(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        )
    }
}

fn open_at(directory: &File, name: &[u8], flags: i32) -> Result<File, ClosureError> {
    let name = CString::new(name).map_err(|_| ClosureError::InvalidPath)?;
    // SAFETY: directory is live, name is one NUL-terminated component, and a
    // successful openat returns one newly owned descriptor.
    let descriptor = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
    if descriptor < 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ENOENT) {
            Err(ClosureError::Missing)
        } else if matches!(error.raw_os_error(), Some(libc::ELOOP | libc::ENOTDIR)) {
            Err(ClosureError::SpecialFile)
        } else {
            Err(ClosureError::Unreadable)
        }
    } else {
        // SAFETY: the successful descriptor is newly owned.
        Ok(unsafe { File::from_raw_fd(descriptor) })
    }
}

fn root_identity(metadata: &fs::Metadata) -> RootIdentity {
    (metadata.dev(), metadata.ino(), metadata.mode())
}
