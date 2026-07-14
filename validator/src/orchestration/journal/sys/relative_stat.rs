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
    let stream = unsafe { libc::fdopendir(descriptor) };
    if stream.is_null() {
        unsafe { libc::close(descriptor) };
        return Err(OrchestrationError::JournalCorrupt);
    }
    let mut exact = 0_u8;
    let mut alias = 0_u8;
    loop {
        clear_readdir_error();
        let entry = unsafe { libc::readdir(stream) };
        if entry.is_null() {
            if readdir_failed() {
                unsafe { libc::closedir(stream) };
                return Err(OrchestrationError::JournalCorrupt);
            }
            break;
        }
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if name == expected.as_bytes() {
            exact = exact.saturating_add(1);
        } else if name.eq_ignore_ascii_case(expected.as_bytes()) {
            alias = alias.saturating_add(1);
        }
    }
    unsafe { libc::closedir(stream) };
    if exact > 1 || alias != 0 {
        return Err(OrchestrationError::JournalCorrupt);
    }
    Ok(exact == 1)
}

#[cfg(target_os = "macos")]
fn clear_readdir_error() {
    unsafe { *libc::__error() = 0 };
}

#[cfg(target_os = "macos")]
fn readdir_failed() -> bool {
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
    let descriptor = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            flags,
            mode as libc::c_uint,
        )
    };
    if descriptor < 0 {
        return Err(match std::io::Error::last_os_error().raw_os_error() {
            Some(libc::EEXIST) => OrchestrationError::JournalConflict,
            _ => OrchestrationError::JournalCorrupt,
        });
    }
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
    let stat = unsafe { stat.assume_init() };
    Ok(Some(RelativeStat {
        device: stat.st_dev as u64,
        inode: stat.st_ino as u64,
        regular: stat.st_mode & libc::S_IFMT == libc::S_IFREG,
        links: stat.st_nlink as u64,
        length: stat.st_size.max(0) as u64,
    }))
}

#[cfg(not(unix))]
pub(crate) fn relative_stat(_: &File, _: &str) -> Result<Option<RelativeStat>, OrchestrationError> {
    Err(OrchestrationError::JournalIo)
}
