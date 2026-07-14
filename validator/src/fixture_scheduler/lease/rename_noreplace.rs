#[cfg(target_os = "linux")]
fn rename_noreplace(from_fd: RawFd, from: &CStr, to_fd: RawFd, to: &CStr) -> io::Result<()> {
    let result = unsafe {
        libc::syscall(
            libc::SYS_renameat2,
            from_fd,
            from.as_ptr(),
            to_fd,
            to.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(all(unix, not(any(target_os = "macos", target_os = "linux"))))]
fn rename_noreplace(_from_fd: RawFd, _from: &CStr, _to_fd: RawFd, _to: &CStr) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "atomic no-replace rename unavailable on this platform",
    ))
}

#[cfg(unix)]
fn open_directory(path: &Path) -> io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
}

#[cfg(unix)]
fn open_directory_at(parent_fd: RawFd, name: &CStr) -> io::Result<OwnedFd> {
    open_at(
        parent_fd,
        name,
        libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
    )
}

#[cfg(unix)]
fn open_entry_at(parent_fd: RawFd, name: &CStr, kind: EntryKind) -> io::Result<OwnedFd> {
    let flags = match kind {
        EntryKind::Directory => {
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC
        }
        EntryKind::RegularFile => {
            libc::O_RDONLY | libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC
        }
    };
    open_at(parent_fd, name, flags)
}

#[cfg(unix)]
fn open_at(parent_fd: RawFd, name: &CStr, flags: libc::c_int) -> io::Result<OwnedFd> {
    let descriptor = unsafe { libc::openat(parent_fd, name.as_ptr(), flags) };
    if descriptor < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(unsafe { OwnedFd::from_raw_fd(descriptor) })
    }
}

#[cfg(unix)]
fn identity_for_fd(fd: RawFd) -> io::Result<EntryIdentity> {
    let mut stat = MaybeUninit::<libc::stat>::zeroed();
    if unsafe { libc::fstat(fd, stat.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    identity_from_stat(unsafe { stat.assume_init() })
}

#[cfg(unix)]
fn entry_identity(parent_fd: RawFd, name: &CStr) -> io::Result<Option<EntryIdentity>> {
    let mut stat = MaybeUninit::<libc::stat>::zeroed();
    let result = unsafe {
        libc::fstatat(
            parent_fd,
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result == 0 {
        identity_from_stat(unsafe { stat.assume_init() }).map(Some)
    } else {
        let error = io::Error::last_os_error();
        if error.kind() == io::ErrorKind::NotFound {
            Ok(None)
        } else {
            Err(error)
        }
    }
}

#[cfg(unix)]
fn identity_from_stat(stat: libc::stat) -> io::Result<EntryIdentity> {
    let file_type = stat.st_mode & libc::S_IFMT;
    let kind = if file_type == libc::S_IFDIR {
        EntryKind::Directory
    } else if file_type == libc::S_IFREG {
        EntryKind::RegularFile
    } else if file_type == libc::S_IFLNK {
        return Err(io::Error::other("lease contains a symbolic link"));
    } else {
        return Err(io::Error::other("lease contains a special file"));
    };
    Ok(EntryIdentity {
        device: stat.st_dev as u64,
        inode: stat.st_ino as u64,
        kind,
    })
}

#[cfg(unix)]
fn require_entry_identity(
    parent_fd: RawFd,
    name: &CStr,
    expected: EntryIdentity,
) -> io::Result<()> {
    match entry_identity(parent_fd, name)? {
        Some(observed) if observed == expected => Ok(()),
        _ => Err(io::Error::other("lease filesystem identity changed")),
    }
}

#[cfg(unix)]
fn read_directory_names(directory_fd: RawFd) -> io::Result<Vec<OsString>> {
    let descriptor = unsafe { libc::dup(directory_fd) };
    if descriptor < 0 {
        return Err(io::Error::last_os_error());
    }
    let directory = unsafe { libc::fdopendir(descriptor) };
    if directory.is_null() {
        let error = io::Error::last_os_error();
        unsafe { libc::close(descriptor) };
        return Err(error);
    }
    unsafe { libc::rewinddir(directory) };

    let mut names = Vec::new();
    loop {
        let entry = unsafe { libc::readdir(directory) };
        if entry.is_null() {
            break;
        }
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if name != b"." && name != b".." {
            names.push(OsString::from_vec(name.to_vec()));
        }
    }
    if unsafe { libc::closedir(directory) } != 0 {
        return Err(io::Error::last_os_error());
    }
    names.sort();
    Ok(names)
}

#[cfg(unix)]
fn component_c_string(value: &OsStr) -> io::Result<CString> {
    if value.as_bytes().contains(&b'/') {
        return Err(io::Error::other("lease entry is not one path component"));
    }
    CString::new(value.as_bytes())
        .map_err(|_| io::Error::other("lease entry contains an invalid NUL"))
}

#[cfg(not(unix))]
fn remove_regular_tree(_path: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "identity-conditioned filesystem deletion is unavailable on this platform; lease retained for recovery",
    ))
}

pub(crate) fn isolated_environment(lease: &IsolationLease) -> BTreeMap<String, String> {
    lease
        .bindings
        .iter()
        .map(|binding| {
            (
                format!("HUL_FIXTURE_{}", binding.kind.label().to_ascii_uppercase()),
                binding.key.clone(),
            )
        })
        .collect()
}
