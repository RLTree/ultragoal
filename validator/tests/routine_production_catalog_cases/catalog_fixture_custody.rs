use std::ffi::{CStr, CString, OsString};
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::MetadataExt;
use std::sync::atomic::{AtomicU64, Ordering};

use super::catalog_fixture_scope::FixtureScopeError;
use crate::catalog_fixture_custody_types::{EntryIdentity, EntryKind};

static NEXT_QUARANTINE: AtomicU64 = AtomicU64::new(0);

pub(crate) enum CleanupDisposition {
    Deleted,
    Retained { name: CString, reason: String },
}

pub(crate) fn quarantine_and_remove(
    parent: &File,
    child: &File,
    name: &CStr,
    device: u64,
    inode: u64,
) -> CleanupDisposition {
    let quarantine = match fresh_quarantine(parent.as_raw_fd()) {
        Ok(name) => name,
        Err(error) => return retained(name, error),
    };
    if let Err(error) = rename_exclusive(parent.as_raw_fd(), name, &quarantine) {
        return retained(name, error);
    }
    if let Err(error) = validate(parent.as_raw_fd(), &quarantine, child, device, inode) {
        return restore_or_retain(parent.as_raw_fd(), name, &quarantine, error);
    }
    if let Err(error) = clear(child.as_raw_fd()) {
        return restore_or_retain(parent.as_raw_fd(), name, &quarantine, error);
    }
    if let Err(error) = validate(parent.as_raw_fd(), &quarantine, child, device, inode) {
        return restore_or_retain(parent.as_raw_fd(), name, &quarantine, error);
    }
    crate::catalog_fixture_cleanup_hook::run_before_final_removal(&quarantine);
    if let Err(error) = validate(parent.as_raw_fd(), &quarantine, child, device, inode) {
        return restore_or_retain(parent.as_raw_fd(), name, &quarantine, error);
    }
    if unsafe { libc::unlinkat(parent.as_raw_fd(), quarantine.as_ptr(), libc::AT_REMOVEDIR) } != 0 {
        return restore_or_retain(
            parent.as_raw_fd(),
            name,
            &quarantine,
            io::Error::last_os_error().to_string(),
        );
    }
    CleanupDisposition::Deleted
}

fn restore_or_retain(
    parent: RawFd,
    original: &CStr,
    quarantine: &CStr,
    reason: impl ToString,
) -> CleanupDisposition {
    if rename_exclusive(parent, quarantine, original).is_ok() {
        retained(original, reason)
    } else {
        retained(quarantine, reason)
    }
}

fn clear(directory: RawFd) -> Result<(), String> {
    for name in names(directory)? {
        let name = CString::new(name.into_vec()).map_err(|_| "entry contains NUL".to_owned())?;
        let observed = entry_identity(directory, &name)?;
        if observed.kind == EntryKind::Directory {
            let child = open_directory(directory, &name)?;
            let held = child.metadata().map_err(|error| error.to_string())?;
            if held.dev() != observed.device || held.ino() != observed.inode {
                return Err("child identity changed before cleanup".to_owned());
            }
            clear(child.as_raw_fd())?;
            if entry_identity(directory, &name)? != observed {
                return Err("child identity changed before removal".to_owned());
            }
            remove(directory, &name, libc::AT_REMOVEDIR)?;
        } else if matches!(observed.kind, EntryKind::File | EntryKind::Symlink) {
            if entry_identity(directory, &name)? != observed {
                return Err("entry identity changed before removal".to_owned());
            }
            remove(directory, &name, 0)?;
        } else {
            if entry_identity(directory, &name)? != observed {
                return Err("special entry changed before removal".to_owned());
            }
            remove(directory, &name, 0)?;
        }
    }
    Ok(())
}

fn validate(
    parent: RawFd,
    name: &CStr,
    child: &File,
    device: u64,
    inode: u64,
) -> Result<(), String> {
    let metadata = entry_identity(parent, name)?;
    let held = child.metadata().map_err(|error| error.to_string())?;
    #[cfg(unix)]
    if metadata.device != device
        || metadata.inode != inode
        || held.dev() != device
        || held.ino() != inode
    {
        return Err("quarantined scope identity changed".to_owned());
    }
    if metadata.kind != EntryKind::Directory || !held.file_type().is_dir() {
        return Err("quarantined scope is not a directory".to_owned());
    }
    Ok(())
}

fn retained(name: &CStr, reason: impl ToString) -> CleanupDisposition {
    CleanupDisposition::Retained {
        name: CString::new(name.to_bytes()).expect("validated C name contains no NUL"),
        reason: reason.to_string(),
    }
}

fn fresh_quarantine(parent: RawFd) -> Result<CString, String> {
    for _ in 0..32 {
        let ordinal = NEXT_QUARANTINE.fetch_add(1, Ordering::Relaxed);
        let name = CString::new(format!(
            ".catalog-fixture-quarantine-{}-{ordinal}",
            std::process::id()
        ))
        .expect("fixed quarantine name contains no NUL");
        if matches!(entry_identity(parent, &name), Err(error) if error.contains("No such file")) {
            return Ok(name);
        }
    }
    Err("no unused catalog fixture quarantine name".to_owned())
}

#[cfg(target_os = "macos")]
fn rename_exclusive(parent: RawFd, from: &CStr, to: &CStr) -> Result<(), String> {
    if unsafe {
        libc::renameatx_np(
            parent,
            from.as_ptr(),
            parent,
            to.as_ptr(),
            libc::RENAME_EXCL,
        )
    } == 0
    {
        Ok(())
    } else {
        Err(io::Error::last_os_error().to_string())
    }
}

#[cfg(not(target_os = "macos"))]
fn rename_exclusive(_parent: RawFd, _from: &CStr, _to: &CStr) -> Result<(), String> {
    Err("exclusive rename unavailable on this platform".to_owned())
}

fn remove(parent: RawFd, name: &CStr, flags: libc::c_int) -> Result<(), String> {
    if unsafe { libc::unlinkat(parent, name.as_ptr(), flags) } == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error().to_string())
    }
}

fn open_directory(parent: RawFd, name: &CStr) -> Result<File, String> {
    let fd = unsafe {
        libc::openat(
            parent,
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        Err(io::Error::last_os_error().to_string())
    } else {
        Ok(unsafe { File::from_raw_fd(fd) })
    }
}

fn entry_identity(parent: RawFd, name: &CStr) -> Result<EntryIdentity, String> {
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    if unsafe {
        libc::fstatat(
            parent,
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } != 0
    {
        return Err(io::Error::last_os_error().to_string());
    }
    let stat = unsafe { stat.assume_init() };
    let kind = match stat.st_mode & libc::S_IFMT {
        libc::S_IFDIR => EntryKind::Directory,
        libc::S_IFREG => EntryKind::File,
        libc::S_IFLNK => EntryKind::Symlink,
        _ => EntryKind::Special,
    };
    Ok(EntryIdentity {
        device: stat.st_dev as u64,
        inode: stat.st_ino as u64,
        kind,
    })
}

fn names(directory: RawFd) -> Result<Vec<OsString>, String> {
    let duplicate = unsafe { libc::dup(directory) };
    if duplicate < 0 {
        return Err(io::Error::last_os_error().to_string());
    }
    let stream = unsafe { libc::fdopendir(duplicate) };
    if stream.is_null() {
        unsafe { libc::close(duplicate) };
        return Err(io::Error::last_os_error().to_string());
    }
    let mut result = Vec::new();
    loop {
        let entry = unsafe { libc::readdir(stream) };
        if entry.is_null() {
            break;
        }
        let bytes = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if bytes != b"." && bytes != b".." {
            result.push(OsString::from_vec(bytes.to_vec()));
        }
    }
    if unsafe { libc::closedir(stream) } != 0 {
        return Err(io::Error::last_os_error().to_string());
    }
    result.sort();
    Ok(result)
}
