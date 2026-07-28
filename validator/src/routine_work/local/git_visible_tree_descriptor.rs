use super::{capture, unsupported};
use crate::routine_work::RoutineError;
use std::ffi::{CStr, CString, OsString};
use std::fs::{self, File, OpenOptions};
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

#[path = "git_visible_tree_test_hook.rs"]
mod test_hook;
#[cfg(test)]
pub(super) fn set_test_before_directory_open(hook: impl FnOnce() + 'static) {
    test_hook::set(hook);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct EntryIdentity {
    pub(super) device: u64,
    pub(super) inode: u64,
    pub(super) mode: u32,
    pub(super) links: u64,
    pub(super) length: u64,
    pub(super) modified_seconds: i64,
    pub(super) modified_nanos: i64,
}

impl EntryIdentity {
    pub(super) fn kind(self) -> Result<u8, RoutineError> {
        let kind = self.mode & u32::from(libc::S_IFMT);
        if kind == u32::from(libc::S_IFLNK) {
            return Err(unsupported("worktree-symlink-not-supported"));
        }
        if kind == u32::from(libc::S_IFDIR) {
            return Ok(b'd');
        }
        if kind == u32::from(libc::S_IFREG) {
            if self.links != 1 {
                return Err(unsupported("worktree-hardlink-not-supported"));
            }
            return Ok(b'f');
        }
        Err(unsupported("worktree-special-entry-not-supported"))
    }
}

pub(super) struct Entry {
    pub(super) name: OsString,
    pub(super) identity: EntryIdentity,
}

pub(super) fn open_root(root: &Path) -> Result<File, RoutineError> {
    let before =
        fs::symlink_metadata(root).map_err(|_| capture("worktree-root-metadata-failed"))?;
    if before.file_type().is_symlink() || !before.is_dir() {
        return Err(capture("worktree-root-unsafe"));
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(root)
        .map_err(|_| capture("worktree-root-open-failed"))?;
    if metadata_identity(&before)
        != metadata_identity(
            &file
                .metadata()
                .map_err(|_| capture("worktree-root-metadata-failed"))?,
        )
    {
        return Err(capture("worktree-root-open-raced"));
    }
    Ok(file)
}

pub(super) fn entries(directory: &File) -> Result<Vec<Entry>, RoutineError> {
    let descriptor = duplicate(directory)?;
    // SAFETY: fdopendir takes ownership of the duplicated directory descriptor.
    let stream = unsafe { libc::fdopendir(descriptor) };
    if stream.is_null() {
        // SAFETY: fdopendir failed, so this branch still owns descriptor.
        unsafe { libc::close(descriptor) };
        return Err(capture("worktree-directory-read-failed"));
    }
    let stream = DirectoryStream(stream);
    let mut entries = Vec::new();
    loop {
        clear_errno();
        // SAFETY: stream is live and owned by DirectoryStream.
        let entry = unsafe { libc::readdir(stream.0) };
        if entry.is_null() {
            if errno() != 0 {
                return Err(capture("worktree-entry-read-failed"));
            }
            break;
        }
        // SAFETY: non-null dirent owns a NUL-terminated d_name until next readdir.
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if matches!(name, b"." | b"..") {
            continue;
        }
        entries.push(Entry {
            name: OsString::from_vec(name.to_vec()),
            identity: stat_at(directory, name)?,
        });
    }
    Ok(entries)
}

pub(super) fn open_child_directory(
    parent: &File,
    name: &OsString,
    expected: EntryIdentity,
) -> Result<File, RoutineError> {
    test_hook::run();
    let name = CString::new(name.as_bytes()).map_err(|_| capture("worktree-path-invalid"))?;
    // SAFETY: parent is live, name is one NUL-terminated component, and a
    // successful openat returns one newly owned descriptor.
    let descriptor = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if descriptor < 0 {
        return Err(capture("worktree-directory-open-raced"));
    }
    // SAFETY: the successful descriptor is newly owned.
    let file = unsafe { File::from_raw_fd(descriptor) };
    let opened = metadata_identity(
        &file
            .metadata()
            .map_err(|_| capture("worktree-entry-metadata-failed"))?,
    );
    if opened != expected || stat_at(parent, name.as_bytes())? != expected {
        return Err(capture("worktree-directory-open-raced"));
    }
    Ok(file)
}

fn stat_at(parent: &File, name: &[u8]) -> Result<EntryIdentity, RoutineError> {
    let name = CString::new(name).map_err(|_| capture("worktree-path-invalid"))?;
    let mut stat = MaybeUninit::<libc::stat>::uninit();
    // SAFETY: parent is live, name is NUL-terminated, and stat is writable.
    if unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } != 0
    {
        return Err(capture("worktree-entry-metadata-failed"));
    }
    // SAFETY: successful fstatat initialized stat.
    let stat = unsafe { stat.assume_init() };
    Ok(stat_identity(&stat))
}

fn duplicate(directory: &File) -> Result<i32, RoutineError> {
    // SAFETY: directory is live and fcntl duplicates it into one owned descriptor.
    let descriptor = unsafe { libc::fcntl(directory.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 0) };
    if descriptor < 0 {
        Err(capture("worktree-directory-read-failed"))
    } else {
        Ok(descriptor)
    }
}

fn metadata_identity(metadata: &fs::Metadata) -> EntryIdentity {
    EntryIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        mode: metadata.mode(),
        links: metadata.nlink(),
        length: metadata.len(),
        modified_seconds: metadata.mtime(),
        modified_nanos: metadata.mtime_nsec(),
    }
}

#[cfg(target_os = "macos")]
fn stat_identity(stat: &libc::stat) -> EntryIdentity {
    EntryIdentity {
        device: stat.st_dev as u64,
        inode: stat.st_ino,
        mode: u32::from(stat.st_mode),
        links: u64::from(stat.st_nlink),
        length: stat.st_size as u64,
        modified_seconds: stat.st_mtime,
        modified_nanos: stat.st_mtime_nsec,
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn stat_identity(stat: &libc::stat) -> EntryIdentity {
    EntryIdentity {
        device: stat.st_dev as u64,
        inode: stat.st_ino,
        mode: stat.st_mode,
        links: stat.st_nlink,
        length: stat.st_size as u64,
        modified_seconds: stat.st_mtime,
        modified_nanos: stat.st_mtime_nsec,
    }
}

struct DirectoryStream(*mut libc::DIR);

impl Drop for DirectoryStream {
    fn drop(&mut self) {
        // SAFETY: DirectoryStream exclusively owns this live stream.
        unsafe { libc::closedir(self.0) };
    }
}

#[cfg(target_os = "macos")]
fn clear_errno() {
    // SAFETY: __error returns current-thread errno storage.
    unsafe { *libc::__error() = 0 };
}

#[cfg(target_os = "macos")]
fn errno() -> i32 {
    // SAFETY: __error returns current-thread errno storage.
    unsafe { *libc::__error() }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn clear_errno() {
    // SAFETY: __errno_location returns current-thread errno storage.
    unsafe { *libc::__errno_location() = 0 };
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn errno() -> i32 {
    // SAFETY: __errno_location returns current-thread errno storage.
    unsafe { *libc::__errno_location() }
}
