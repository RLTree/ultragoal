use super::*;

pub(crate) fn directory_is_empty(directory: &File) -> Result<bool, FitError> {
    let descriptor = unsafe { libc::fcntl(directory.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 0) };
    if descriptor < 0 {
        return Err(error(FitErrorId::ReadFailed));
    }
    let stream = unsafe { libc::fdopendir(descriptor) };
    if stream.is_null() {
        unsafe { libc::close(descriptor) };
        return Err(error(FitErrorId::ReadFailed));
    }
    unsafe { libc::rewinddir(stream) };
    let mut entries = 0usize;
    let mut name_bytes = 0usize;
    let result = loop {
        unsafe { *libc::__error() = 0 };
        let entry = unsafe { libc::readdir(stream) };
        if entry.is_null() {
            break if unsafe { *libc::__error() } == 0 {
                Ok(true)
            } else {
                Err(error(FitErrorId::ReadFailed))
            };
        }
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        entries = entries.saturating_add(1);
        name_bytes = name_bytes.saturating_add(name.len());
        if entries > 256 || name_bytes > 64 * 1024 {
            break Err(error(FitErrorId::ResourceLimit));
        }
        if !matches!(name, b"." | b"..") {
            break Ok(false);
        }
    };
    let closed = unsafe { libc::closedir(stream) };
    if closed != 0 {
        Err(error(FitErrorId::ReadFailed))
    } else {
        result
    }
}

pub(crate) fn mkdir_at(directory: &File, name: &str, mode: u32) -> Result<(), FitError> {
    let name = CString::new(name).map_err(|_| error(FitErrorId::InvalidPath))?;
    if unsafe { libc::mkdirat(directory.as_raw_fd(), name.as_ptr(), mode as libc::mode_t) } == 0 {
        Ok(())
    } else {
        Err(error(
            match std::io::Error::last_os_error().raw_os_error() {
                Some(libc::EEXIST) => FitErrorId::Conflict,
                _ => FitErrorId::EffectFailed,
            },
        ))
    }
}

pub(crate) fn create_file_at(directory: &File, name: &str, mode: u32) -> Result<File, FitError> {
    let name = CString::new(name).map_err(|_| error(FitErrorId::InvalidPath))?;
    let descriptor = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            mode,
        )
    };
    if descriptor < 0 {
        return Err(error(FitErrorId::EffectFailed));
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

pub(crate) fn rename_exclusive(
    directory: &File,
    source: &str,
    target: &str,
) -> Result<(), FitError> {
    rename(directory, source, target, libc::RENAME_EXCL)
}

pub(crate) fn rename_swap(directory: &File, source: &str, target: &str) -> Result<(), FitError> {
    rename(directory, source, target, libc::RENAME_SWAP)
}

pub(crate) fn rename(
    directory: &File,
    source: &str,
    target: &str,
    flags: u32,
) -> Result<(), FitError> {
    rename_between(directory, source, directory, target, flags)
}

pub(crate) fn rename_between(
    source_directory: &File,
    source: &str,
    target_directory: &File,
    target: &str,
    flags: u32,
) -> Result<(), FitError> {
    let source = CString::new(source).map_err(|_| error(FitErrorId::InvalidPath))?;
    let target = CString::new(target).map_err(|_| error(FitErrorId::InvalidPath))?;
    if unsafe {
        libc::renameatx_np(
            source_directory.as_raw_fd(),
            source.as_ptr(),
            target_directory.as_raw_fd(),
            target.as_ptr(),
            flags,
        )
    } == 0
    {
        Ok(())
    } else {
        Err(error(
            match std::io::Error::last_os_error().raw_os_error() {
                Some(libc::EEXIST | libc::ENOENT) => FitErrorId::Conflict,
                Some(libc::EXDEV) => FitErrorId::UnsafeObject,
                Some(libc::ENOTSUP | libc::ENOSYS) => FitErrorId::UnsupportedHost,
                _ => FitErrorId::EffectFailed,
            },
        ))
    }
}

pub(crate) fn unlink_at(directory: &File, name: &str, flags: i32) -> Result<(), FitError> {
    let name = CString::new(name).map_err(|_| error(FitErrorId::InvalidPath))?;
    if unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), flags) } == 0 {
        Ok(())
    } else {
        Err(error(
            match std::io::Error::last_os_error().raw_os_error() {
                Some(libc::ENOENT | libc::ENOTEMPTY | libc::EEXIST) => FitErrorId::Conflict,
                _ => FitErrorId::EffectFailed,
            },
        ))
    }
}

pub(crate) fn descriptor_path(file: &File) -> Result<PathBuf, FitError> {
    let mut buffer = [0 as libc::c_char; libc::PATH_MAX as usize];
    if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETPATH, buffer.as_mut_ptr()) } < 0 {
        return Err(error(
            match std::io::Error::last_os_error().raw_os_error() {
                Some(libc::EINVAL | libc::ENOTSUP | libc::ENOSYS) => FitErrorId::UnsupportedHost,
                _ => FitErrorId::StaleBinding,
            },
        ));
    }
    let end = buffer
        .iter()
        .position(|byte| *byte == 0)
        .ok_or_else(|| error(FitErrorId::StaleBinding))?;
    let bytes = buffer[..end]
        .iter()
        .map(|byte| *byte as u8)
        .collect::<Vec<_>>();
    if bytes.first() != Some(&b'/') {
        return Err(error(FitErrorId::StaleBinding));
    }
    Ok(PathBuf::from(OsString::from_vec(bytes)))
}
