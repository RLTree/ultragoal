use std::ffi::{CStr, CString};
use std::fs::File;
use std::io::{self, Write};
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use super::owned_compile_scratch::{
    FAILURE_MARKER, OwnedCompileScratch, SUBSTITUTION_MARKER, SUBSTITUTION_MARKER_NAME,
};

pub(crate) fn cleanup(scratch: &OwnedCompileScratch) {
    let panicking = std::thread::panicking();
    let Some(quarantine) = quarantine_owned_entry(scratch) else {
        retain_substituted(scratch, panicking);
        return;
    };
    let cleared = clear_directory(scratch.directory.as_raw_fd());
    if cleared.is_ok()
        && entry_identity(scratch.parent.as_raw_fd(), &quarantine)
            == Some((scratch.device, scratch.inode))
        && unsafe {
            libc::unlinkat(
                scratch.parent.as_raw_fd(),
                quarantine.as_ptr(),
                libc::AT_REMOVEDIR,
            )
        } == 0
    {
        if panicking {
            let _ = write_new_file_at_path(
                scratch.parent.as_raw_fd(),
                &scratch.failure_marker,
                FAILURE_MARKER,
            );
        }
        return;
    }
    let _ = write_new_file(
        scratch.directory.as_raw_fd(),
        SUBSTITUTION_MARKER_NAME,
        SUBSTITUTION_MARKER,
    );
    if !panicking {
        assert!(cleared.is_ok(), "quarantined scratch cleanup failed");
    }
}

fn quarantine_owned_entry(scratch: &OwnedCompileScratch) -> Option<CString> {
    let quarantine = random_quarantine_name();
    if unsafe {
        libc::renameatx_np(
            scratch.parent.as_raw_fd(),
            scratch.name.as_ptr(),
            scratch.parent.as_raw_fd(),
            quarantine.as_ptr(),
            libc::RENAME_EXCL,
        )
    } != 0
    {
        return None;
    }
    if entry_identity(scratch.parent.as_raw_fd(), &quarantine)
        == Some((scratch.device, scratch.inode))
    {
        return Some(quarantine);
    }
    let restored = unsafe {
        libc::renameatx_np(
            scratch.parent.as_raw_fd(),
            quarantine.as_ptr(),
            scratch.parent.as_raw_fd(),
            scratch.name.as_ptr(),
            libc::RENAME_EXCL,
        )
    };
    assert_eq!(
        restored, 0,
        "replacement scratch entry could not be restored"
    );
    None
}

fn retain_substituted(scratch: &OwnedCompileScratch, panicking: bool) {
    let _ = write_new_file(
        scratch.directory.as_raw_fd(),
        SUBSTITUTION_MARKER_NAME,
        SUBSTITUTION_MARKER,
    );
    if !panicking {
        assert!(bounded_tree(scratch.directory.as_raw_fd(), 256 * 1024).unwrap_or(false));
    }
}

fn random_quarantine_name() -> CString {
    let mut nonce = [0_u8; 16];
    getrandom::fill(&mut nonce).expect("scratch quarantine randomness unavailable");
    CString::new(format!(".routine-cleanup-{}", hex(&nonce))).unwrap()
}

fn hex(bytes: &[u8]) -> String {
    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut value, "{byte:02x}").unwrap();
    }
    value
}

pub(crate) fn open_directory_at(parent: RawFd, name: &CStr) -> io::Result<File> {
    let fd = unsafe {
        libc::openat(
            parent,
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(unsafe { File::from_raw_fd(fd) })
    }
}

fn entry_identity(parent: RawFd, name: &CStr) -> Option<(u64, u64)> {
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            parent,
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    (result == 0).then(|| {
        let stat = unsafe { stat.assume_init() };
        (stat.st_dev as u64, stat.st_ino as u64)
    })
}

fn clear_directory(directory: RawFd) -> io::Result<()> {
    for name in directory_entries(directory)? {
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        if unsafe {
            libc::fstatat(
                directory,
                name.as_ptr(),
                stat.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } != 0
        {
            return Err(io::Error::last_os_error());
        }
        let stat = unsafe { stat.assume_init() };
        let is_directory = stat.st_mode & libc::S_IFMT == libc::S_IFDIR;
        if is_directory {
            let child = open_directory_at(directory, &name)?;
            clear_directory(child.as_raw_fd())?;
        }
        let flags = if is_directory { libc::AT_REMOVEDIR } else { 0 };
        if unsafe { libc::unlinkat(directory, name.as_ptr(), flags) } != 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

fn bounded_tree(directory: RawFd, limit: u64) -> io::Result<bool> {
    let mut total = 0_u64;
    for name in directory_entries(directory)? {
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        if unsafe {
            libc::fstatat(
                directory,
                name.as_ptr(),
                stat.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } != 0
        {
            return Err(io::Error::last_os_error());
        }
        total = total.saturating_add(unsafe { stat.assume_init() }.st_size.max(0) as u64);
        if total > limit {
            return Ok(false);
        }
    }
    Ok(true)
}

fn directory_entries(directory: RawFd) -> io::Result<Vec<CString>> {
    let duplicate = unsafe { libc::dup(directory) };
    if duplicate < 0 {
        return Err(io::Error::last_os_error());
    }
    let stream = unsafe { libc::fdopendir(duplicate) };
    if stream.is_null() {
        unsafe { libc::close(duplicate) };
        return Err(io::Error::last_os_error());
    }
    let mut names = Vec::new();
    loop {
        let entry = unsafe { libc::readdir(stream) };
        if entry.is_null() {
            break;
        }
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) };
        if name.to_bytes() != b"." && name.to_bytes() != b".." {
            names.push(CString::new(name.to_bytes()).unwrap());
        }
    }
    if unsafe { libc::closedir(stream) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(names)
}

pub(crate) fn write_new_file(directory: RawFd, name: &str, bytes: &[u8]) -> io::Result<()> {
    let name = CString::new(name).unwrap();
    let fd = unsafe {
        libc::openat(
            directory,
            name.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC,
            0o600,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    unsafe { File::from_raw_fd(fd) }.write_all(bytes)
}

fn write_new_file_at_path(directory: RawFd, path: &Path, bytes: &[u8]) -> io::Result<()> {
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "marker name missing"))?;
    let name = CString::new(name.as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "marker name invalid"))?;
    write_new_file(directory, name.to_str().unwrap(), bytes)
}
