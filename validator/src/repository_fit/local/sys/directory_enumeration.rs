use super::*;

pub(crate) fn enumerate<S: EntrySource>(
    entries: &mut S,
    expected: &str,
    budget: &mut EnumerationBudget,
) -> Result<EntryMatch, FitError> {
    let mut names = EntryNames::default();
    let mut entry_count: usize = 0;
    let mut name_bytes: usize = 0;
    loop {
        let observation = entries.next();
        let entry_name_bytes = match &observation {
            Ok(None) => break,
            Ok(Some(name)) => name.len(),
            Err(_) => 0,
        };
        entry_count = entry_count
            .checked_add(1)
            .ok_or_else(|| error(FitErrorId::ResourceLimit))?;
        name_bytes = name_bytes
            .checked_add(entry_name_bytes)
            .ok_or_else(|| error(FitErrorId::ResourceLimit))?;
        budget.observe(entry_count, name_bytes, entry_name_bytes)?;
        let name = observation?;
        let Some(name) = name else {
            unreachable!("normal EOF is handled before budget accounting")
        };
        let name = std::str::from_utf8(name).map_err(|_| error(FitErrorId::UnsafeObject))?;
        names.observe(expected, name);
    }
    Ok(names.classify())
}

#[cfg(target_vendor = "apple")]
pub(crate) fn errno_location() -> *mut libc::c_int {
    unsafe { libc::__error() }
}

#[cfg(target_os = "linux")]
pub(crate) fn errno_location() -> *mut libc::c_int {
    unsafe { libc::__errno_location() }
}

#[cfg(any(target_vendor = "apple", target_os = "linux"))]
pub(crate) fn clear_readdir_error() -> Result<(), FitError> {
    unsafe { *errno_location() = 0 };
    Ok(())
}

#[cfg(any(target_vendor = "apple", target_os = "linux"))]
pub(crate) fn readdir_error() -> Result<i32, FitError> {
    Ok(unsafe { *errno_location() })
}

#[cfg(not(any(target_vendor = "apple", target_os = "linux")))]
pub(crate) fn clear_readdir_error() -> Result<(), FitError> {
    Err(error(FitErrorId::UnsupportedHost))
}

#[cfg(not(any(target_vendor = "apple", target_os = "linux")))]
pub(crate) fn readdir_error() -> Result<i32, FitError> {
    Err(error(FitErrorId::UnsupportedHost))
}

pub(crate) fn duplicate(file: &File) -> Result<File, FitError> {
    let descriptor = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 0) };
    if descriptor < 0 {
        return Err(error(FitErrorId::ReadFailed));
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

pub(crate) fn open_at(directory: &File, name: &str, flags: i32) -> Result<Option<File>, FitError> {
    let name = CString::new(name).map_err(|_| error(FitErrorId::InvalidPath))?;
    let descriptor = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
    if descriptor < 0 {
        return match std::io::Error::last_os_error().raw_os_error() {
            Some(libc::ENOENT) => Ok(None),
            _ => Err(error(FitErrorId::UnsafeObject)),
        };
    }
    Ok(Some(unsafe { File::from_raw_fd(descriptor) }))
}

pub(crate) fn stat_at(directory: &File, name: &str) -> Result<Option<PathStat>, FitError> {
    let name = CString::new(name).map_err(|_| error(FitErrorId::InvalidPath))?;
    let mut stat = MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            directory.as_raw_fd(),
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result != 0 {
        return match std::io::Error::last_os_error().raw_os_error() {
            Some(libc::ENOENT) => Ok(None),
            _ => Err(error(FitErrorId::ReadFailed)),
        };
    }
    let stat = unsafe { stat.assume_init() };
    Ok(Some(PathStat {
        device: stat.st_dev as u64,
        inode: stat.st_ino as u64,
        links: stat.st_nlink as u64,
        length: stat.st_size as u64,
        modified_seconds: stat.st_mtime,
        modified_nanoseconds: stat.st_mtime_nsec,
        changed_seconds: stat.st_ctime,
        changed_nanoseconds: stat.st_ctime_nsec,
        regular: stat.st_mode & libc::S_IFMT == libc::S_IFREG,
    }))
}
