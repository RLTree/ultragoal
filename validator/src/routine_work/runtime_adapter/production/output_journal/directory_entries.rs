use super::*;

pub(super) fn names(directory: &File) -> Result<BTreeSet<String>, RoutineError> {
    // SAFETY: `directory` owns a live descriptor, which `dup` copies.
    let fd = unsafe { libc::dup(directory.as_raw_fd()) };
    if fd < 0 {
        return Err(error("routine-production-output-directory-dup-failed"));
    }
    // SAFETY: `fd` is a newly owned directory descriptor on success.
    let stream = unsafe { libc::fdopendir(fd) };
    if stream.is_null() {
        // SAFETY: `fdopendir` failed, so `fd` remains owned by this function.
        unsafe { libc::close(fd) };
        return Err(error("routine-production-output-directory-open-failed"));
    }
    let mut names = BTreeSet::new();
    loop {
        // SAFETY: `stream` is live; Darwin exposes errno storage through `__error`.
        unsafe { *libc::__error() = 0 };
        // SAFETY: `stream` stays live until `closedir` below.
        let entry = unsafe { libc::readdir(stream) };
        if entry.is_null() {
            let failed = std::io::Error::last_os_error().raw_os_error().unwrap_or(0) != 0;
            // SAFETY: `stream` owns `fd` and is closed exactly once on this path.
            unsafe { libc::closedir(stream) };
            return if failed {
                Err(error("routine-production-output-directory-read-failed"))
            } else {
                Ok(names)
            };
        }
        // SAFETY: a non-null directory entry has a NUL-terminated name valid until
        // the next directory operation, and the bytes are consumed immediately.
        let bytes = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if bytes == b"." || bytes == b".." {
            continue;
        }
        let name = std::str::from_utf8(bytes)
            .map_err(|_| error("routine-production-output-entry-name-invalid"))?;
        names.insert(name.to_owned());
    }
}
