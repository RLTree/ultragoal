use super::{RedCatalogError, error};
use std::ffi::CString;
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

#[path = "filesystem_directory.rs"]
mod directory;

const MAX_RED_FILE_BYTES: u64 = 16 * 1024 * 1024;
type DirectoryIdentity = (u64, u64, u32);
pub(super) type DirectorySnapshot = (u64, u64, u32, u64, u64, i64, i64, i64, i64);

pub(super) fn directory_names(
    directory_file: &File,
) -> Result<Vec<std::ffi::OsString>, RedCatalogError> {
    directory::directory_names(directory_file)
}

pub(super) struct BoundRoot {
    file: File,
    path: PathBuf,
    identity: DirectoryIdentity,
}

impl BoundRoot {
    pub(super) fn open(root: &Path) -> Result<Self, RedCatalogError> {
        let before = fs::symlink_metadata(root)
            .map_err(|_| error("red_catalog_root_unreadable", "repository"))?;
        if before.file_type().is_symlink() || !before.is_dir() {
            return Err(error("red_catalog_root_not_regular", "repository"));
        }
        let path = fs::canonicalize(root)
            .map_err(|_| error("red_catalog_root_unreadable", "repository"))?;
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&path)
            .map_err(|_| error("red_catalog_root_unreadable", "repository"))?;
        let opened = file
            .metadata()
            .map_err(|_| error("red_catalog_root_unreadable", "repository"))?;
        let current = fs::symlink_metadata(&path)
            .map_err(|_| error("red_catalog_root_unreadable", "repository"))?;
        let identity = directory_identity(&before);
        if identity != directory_identity(&opened)
            || identity != directory_identity(&current)
            || current.file_type().is_symlink()
        {
            return Err(error("red_catalog_root_changed", "repository"));
        }
        Ok(Self {
            file,
            path,
            identity,
        })
    }

    pub(super) fn file(&self) -> &File {
        &self.file
    }

    pub(super) fn validate(&self) -> Result<(), RedCatalogError> {
        let current = fs::symlink_metadata(&self.path)
            .map_err(|_| error("red_catalog_root_changed", "repository"))?;
        if current.file_type().is_symlink() || directory_identity(&current) != self.identity {
            return Err(error("red_catalog_root_changed", "repository"));
        }
        let reopened = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&self.path)
            .map_err(|_| error("red_catalog_root_changed", "repository"))?;
        if directory_identity(
            &reopened
                .metadata()
                .map_err(|_| error("red_catalog_root_changed", "repository"))?,
        ) != self.identity
        {
            return Err(error("red_catalog_root_changed", "repository"));
        }
        Ok(())
    }
}

pub(super) fn open_directory(
    parent: &File,
    name: &[u8],
    detail: &str,
) -> Result<File, RedCatalogError> {
    let name = CString::new(name).map_err(|_| error("red_catalog_fixture_path_invalid", detail))?;
    open_at(
        parent,
        &name,
        libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
    )
    .map_err(|_| error("red_catalog_fixture_directory_unreadable", detail))
}

pub(super) fn read_regular(
    parent: &File,
    name: &[u8],
    detail: &str,
) -> Result<Vec<u8>, RedCatalogError> {
    let name = CString::new(name).map_err(|_| error("red_catalog_fixture_path_invalid", detail))?;
    let mut file = open_at(
        parent,
        &name,
        libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
    )
    .map_err(|error_value| {
        if error_value.raw_os_error() == Some(libc::ELOOP) {
            error("red_catalog_regular_file_required", detail)
        } else {
            error("red_catalog_regular_file_unreadable", detail)
        }
    })?;
    let before = file
        .metadata()
        .map_err(|_| error("red_catalog_regular_file_unreadable", detail))?;
    if !before.is_file() || before.nlink() != 1 || before.len() > MAX_RED_FILE_BYTES {
        return Err(error("red_catalog_regular_file_required", detail));
    }
    let mut bytes = Vec::new();
    file.by_ref()
        .take(MAX_RED_FILE_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| error("red_catalog_regular_file_unreadable", detail))?;
    let after = file
        .metadata()
        .map_err(|_| error("red_catalog_regular_file_unreadable", detail))?;
    let current = open_at(
        parent,
        &name,
        libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
    )
    .map_err(|_| error("red_catalog_regular_file_unreadable", detail))?
    .metadata()
    .map_err(|_| error("red_catalog_regular_file_unreadable", detail))?;
    if bytes.len() as u64 > MAX_RED_FILE_BYTES
        || bytes.len() as u64 != before.len()
        || file_identity(&before) != file_identity(&after)
        || file_identity(&before) != file_identity(&current)
    {
        return Err(error("red_catalog_regular_file_changed", detail));
    }
    Ok(bytes)
}

pub(super) fn directory_snapshot(directory: &File) -> Result<DirectorySnapshot, RedCatalogError> {
    let metadata = directory
        .metadata()
        .map_err(|_| error("red_catalog_fixture_directory_unreadable", "fixtures/red"))?;
    Ok((
        metadata.dev(),
        metadata.ino(),
        metadata.mode(),
        metadata.nlink(),
        metadata.len(),
        metadata.mtime(),
        metadata.mtime_nsec(),
        metadata.ctime(),
        metadata.ctime_nsec(),
    ))
}

fn open_at(parent: &File, name: &CString, flags: i32) -> std::io::Result<File> {
    // SAFETY: parent is live, name is NUL-terminated, and a successful openat
    // returns one newly owned descriptor.
    let descriptor = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if descriptor < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        // SAFETY: the successful descriptor is newly owned.
        Ok(unsafe { File::from_raw_fd(descriptor) })
    }
}

fn directory_identity(metadata: &fs::Metadata) -> DirectoryIdentity {
    (metadata.dev(), metadata.ino(), metadata.mode())
}

fn file_identity(metadata: &fs::Metadata) -> (u64, u64, u32, u64, u64, i64, i64, i64, i64) {
    (
        metadata.dev(),
        metadata.ino(),
        metadata.mode(),
        metadata.nlink(),
        metadata.len(),
        metadata.mtime(),
        metadata.mtime_nsec(),
        metadata.ctime(),
        metadata.ctime_nsec(),
    )
}
