use super::*;
use std::collections::BTreeSet;

impl AnchoredDirectory {
    pub(crate) fn entry_names(&self) -> Result<BTreeSet<String>, HostFailure> {
        let current = c".";
        // SAFETY: the parent descriptor is live and the static name is
        // NUL-terminated. Reopening `.` creates an independent directory
        // cursor; `dup` would share and exhaust the anchored descriptor's
        // cursor across successive scans.
        let descriptor = unsafe {
            libc::openat(
                self.file.as_raw_fd(),
                current.as_ptr(),
                libc::O_RDONLY
                    | libc::O_DIRECTORY
                    | libc::O_CLOEXEC
                    | libc::O_NOFOLLOW
                    | libc::O_NONBLOCK,
                0,
            )
        };
        if descriptor < 0 {
            return Err(HostFailure::Invalid);
        }
        // SAFETY: `descriptor` is exclusively owned here and is consumed by `closedir`.
        let directory = unsafe { libc::fdopendir(descriptor) };
        if directory.is_null() {
            // SAFETY: fdopendir did not consume the descriptor on failure.
            unsafe { libc::close(descriptor) };
            return Err(HostFailure::Invalid);
        }
        let mut names = BTreeSet::new();
        let outcome = loop {
            // SAFETY: __error returns this thread's errno storage on the supported host.
            unsafe { *libc::__error() = 0 };
            // SAFETY: the directory stream is live until the matching `closedir` below.
            let entry = unsafe { libc::readdir(directory) };
            if entry.is_null() {
                break match std::io::Error::last_os_error().raw_os_error() {
                    Some(0) | None => Ok(()),
                    _ => Err(HostFailure::Invalid),
                };
            }
            // SAFETY: readdir returned a non-null entry with a NUL-terminated name.
            let name = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) };
            let Ok(name) = name.to_str() else {
                break Err(HostFailure::Invalid);
            };
            if name != "." && name != ".." {
                names.insert(name.to_owned());
            }
        };
        // SAFETY: this closes the stream and its owned descriptor exactly once.
        let closed = unsafe { libc::closedir(directory) };
        outcome?;
        if closed != 0 {
            return Err(HostFailure::Invalid);
        }
        self.verify()?;
        Ok(names)
    }
}
