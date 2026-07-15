use super::*;

pub(super) fn names(directory: &File) -> Result<BTreeSet<String>, RoutineError> {
    let fd = unsafe { libc::dup(directory.as_raw_fd()) };
    if fd < 0 {
        return Err(error("routine-production-output-directory-dup-failed"));
    }
    let stream = unsafe { libc::fdopendir(fd) };
    if stream.is_null() {
        unsafe { libc::close(fd) };
        return Err(error("routine-production-output-directory-open-failed"));
    }
    let mut names = BTreeSet::new();
    loop {
        unsafe { *libc::__error() = 0 };
        let entry = unsafe { libc::readdir(stream) };
        if entry.is_null() {
            let failed = std::io::Error::last_os_error().raw_os_error().unwrap_or(0) != 0;
            unsafe { libc::closedir(stream) };
            return if failed {
                Err(error("routine-production-output-directory-read-failed"))
            } else {
                Ok(names)
            };
        }
        let bytes = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if bytes == b"." || bytes == b".." {
            continue;
        }
        let name = std::str::from_utf8(bytes)
            .map_err(|_| error("routine-production-output-entry-name-invalid"))?;
        names.insert(name.to_owned());
    }
}
