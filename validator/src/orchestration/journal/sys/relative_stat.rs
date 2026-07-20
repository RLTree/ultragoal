#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RelativeStat {
    pub device: u64,
    pub inode: u64,
    pub regular: bool,
    pub links: u64,
    pub length: u64,
}

pub(crate) fn validate_directory(metadata: &fs::Metadata) -> Result<(), OrchestrationError> {
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(OrchestrationError::JournalCorrupt);
    }
    Ok(())
}

pub(crate) fn validate_regular(
    metadata: &fs::Metadata,
    limit: u64,
) -> Result<(), OrchestrationError> {
    if !metadata.is_file() || metadata.len() > limit {
        return Err(OrchestrationError::JournalCorrupt);
    }
    #[cfg(unix)]
    if metadata.nlink() != 1 {
        return Err(OrchestrationError::JournalCorrupt);
    }
    Ok(())
}

pub(crate) fn same_file(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        left.dev() == right.dev() && left.ino() == right.ino()
    }
    #[cfg(not(unix))]
    {
        left.len() == right.len()
    }
}

pub(crate) fn same_named_file(metadata: &fs::Metadata, named: &RelativeStat) -> bool {
    #[cfg(unix)]
    {
        metadata.dev() == named.device && metadata.ino() == named.inode
    }
    #[cfg(not(unix))]
    {
        let _ = (metadata, named);
        false
    }
}

#[cfg(unix)]
pub(crate) fn exact_entry(directory: &File, expected: &str) -> Result<bool, OrchestrationError> {
    let expected = relative_name(expected)?;
    let dot = CString::new(".").expect("static directory component");
    // SAFETY: `directory` owns a live descriptor and `dot` is a static,
    // NUL-terminated relative path.
    let descriptor = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            dot.as_ptr(),
            libc::O_RDONLY
                | libc::O_DIRECTORY
                | libc::O_NOFOLLOW
                | libc::O_CLOEXEC
                | libc::O_NONBLOCK,
        )
    };
    if descriptor < 0 {
        return Err(OrchestrationError::JournalCorrupt);
    }
    // SAFETY: `descriptor` is an owned directory descriptor on this path;
    // ownership transfers to `fdopendir` when it succeeds.
    let stream = unsafe { libc::fdopendir(descriptor) };
    if stream.is_null() {
        // SAFETY: `fdopendir` did not take ownership on failure, so this closes
        // the still-owned descriptor exactly once.
        unsafe { libc::close(descriptor) };
        return Err(OrchestrationError::JournalCorrupt);
    }
    let mut exact = 0_u8;
    let mut alias = 0_u8;
    loop {
        clear_readdir_error();
        // SAFETY: `stream` remains valid until one of the `closedir` calls
        // below, and `readdir` only borrows it.
        let entry = unsafe { libc::readdir(stream) };
        if entry.is_null() {
            if readdir_failed() {
                // SAFETY: this is the first close of the live directory stream.
                unsafe { libc::closedir(stream) };
                return Err(OrchestrationError::JournalCorrupt);
            }
            break;
        }
        // SAFETY: a non-null `readdir` result points to a live `dirent` until
        // the next directory operation; `d_name` is NUL-terminated by libc.
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if name == expected.as_bytes() {
            exact = exact.saturating_add(1);
        } else if name.eq_ignore_ascii_case(expected.as_bytes()) {
            alias = alias.saturating_add(1);
        }
    }
    // SAFETY: this is the first close of the live directory stream.
    unsafe { libc::closedir(stream) };
    if exact > 1 || alias != 0 {
        return Err(OrchestrationError::JournalCorrupt);
    }
    Ok(exact == 1)
}

#[cfg(target_os = "macos")]
fn clear_readdir_error() {
    // SAFETY: macOS exposes the calling thread's writable errno location.
    unsafe { *libc::__error() = 0 };
}

#[cfg(target_os = "macos")]
fn readdir_failed() -> bool {
    // SAFETY: macOS exposes the calling thread's readable errno location.
    unsafe { *libc::__error() != 0 }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn clear_readdir_error() {}

#[cfg(all(unix, not(target_os = "macos")))]
fn readdir_failed() -> bool {
    false
}

#[cfg(not(unix))]
pub(crate) fn exact_entry(_: &File, _: &str) -> Result<bool, OrchestrationError> {
    Err(OrchestrationError::JournalIo)
}

#[cfg(unix)]
pub(crate) fn open_relative(
    directory: &File,
    name: &str,
    flags: i32,
    mode: libc::mode_t,
) -> Result<File, OrchestrationError> {
    let name = relative_name(name)?;
    let mode = libc::c_uint::from(mode);
    // SAFETY: `directory` owns a live directory descriptor, `name` is a
    // NUL-terminated relative path, and `mode` has the ABI type for `openat`.
    let descriptor = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            flags,
            mode,
        )
    };
    if descriptor < 0 {
        return Err(match std::io::Error::last_os_error().raw_os_error() {
            Some(libc::EEXIST) => OrchestrationError::JournalConflict,
            _ => OrchestrationError::JournalCorrupt,
        });
    }
    // SAFETY: successful `openat` returns one owned file descriptor.
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

#[cfg(not(unix))]
pub(crate) fn open_relative(_: &File, _: &str, _: i32, _: u32) -> Result<File, OrchestrationError> {
    Err(OrchestrationError::JournalIo)
}

#[cfg(unix)]
pub(crate) fn relative_stat(
    directory: &File,
    name: &str,
) -> Result<Option<RelativeStat>, OrchestrationError> {
    let name = relative_name(name)?;
    let mut stat = MaybeUninit::<libc::stat>::uninit();
    // SAFETY: `directory` owns a live descriptor, `name` is NUL-terminated,
    // and `stat` is valid writable storage for `fstatat`.
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
            _ => Err(OrchestrationError::JournalCorrupt),
        };
    }
    // SAFETY: successful `fstatat` initialized the complete stat value.
    let stat = unsafe { stat.assume_init() };
    Ok(Some(RelativeStat {
        device: u64::try_from(stat.st_dev).map_err(|_| OrchestrationError::JournalCorrupt)?,
        inode: stat.st_ino,
        regular: stat.st_mode & libc::S_IFMT == libc::S_IFREG,
        links: u64::from(stat.st_nlink),
        length: u64::try_from(stat.st_size).map_err(|_| OrchestrationError::JournalCorrupt)?,
    }))
}

#[cfg(not(unix))]
pub(crate) fn relative_stat(_: &File, _: &str) -> Result<Option<RelativeStat>, OrchestrationError> {
    Err(OrchestrationError::JournalIo)
}
