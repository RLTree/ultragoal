#[cfg(unix)]
pub(crate) fn rename_relative(
    directory: &File,
    from: &str,
    to: &str,
) -> Result<(), OrchestrationError> {
    let from = relative_name(from)?;
    let to = relative_name(to)?;
    // SAFETY: `directory` owns a live descriptor, and both validated names are
    // NUL-terminated single path components resolved relative to that descriptor.
    if unsafe {
        libc::renameat(
            directory.as_raw_fd(),
            from.as_ptr(),
            directory.as_raw_fd(),
            to.as_ptr(),
        )
    } != 0
    {
        return Err(OrchestrationError::JournalIo);
    }
    Ok(())
}

#[cfg(not(unix))]
pub(crate) fn rename_relative(_: &File, _: &str, _: &str) -> Result<(), OrchestrationError> {
    Err(OrchestrationError::JournalIo)
}

#[cfg(unix)]
pub(crate) fn unlink_relative(directory: &File, name: &str) -> Result<(), OrchestrationError> {
    let name = relative_name(name)?;
    // SAFETY: `directory` owns a live descriptor and `name` is a validated,
    // NUL-terminated single path component resolved relative to it.
    if unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) } != 0 {
        return Err(OrchestrationError::JournalIo);
    }
    Ok(())
}

#[cfg(not(unix))]
pub(crate) fn unlink_relative(_: &File, _: &str) -> Result<(), OrchestrationError> {
    Err(OrchestrationError::JournalIo)
}

#[cfg(unix)]
fn relative_name(name: &str) -> Result<CString, OrchestrationError> {
    if name.is_empty() || name.contains('/') || name == "." || name == ".." {
        return Err(OrchestrationError::JournalCorrupt);
    }
    CString::new(name).map_err(|_| OrchestrationError::JournalCorrupt)
}
