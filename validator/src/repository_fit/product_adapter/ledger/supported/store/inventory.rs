use super::*;

impl Store {
    pub(crate) fn names(&self) -> Result<BTreeSet<String>, LedgerError> {
        self.verify_root()?;
        let descriptor = unsafe {
            libc::openat(
                self.directory.as_raw_fd(),
                c".".as_ptr(),
                libc::O_RDONLY
                    | libc::O_DIRECTORY
                    | libc::O_CLOEXEC
                    | libc::O_NOFOLLOW
                    | libc::O_NONBLOCK,
            )
        };
        if descriptor < 0 {
            return Err(ledger_io());
        }
        let stream = unsafe { libc::fdopendir(descriptor) };
        if stream.is_null() {
            unsafe { libc::close(descriptor) };
            return Err(ledger_io());
        }
        let mut names = BTreeSet::new();
        let result = loop {
            unsafe { *libc::__error() = 0 };
            let entry = unsafe { libc::readdir(stream) };
            if entry.is_null() {
                break if unsafe { *libc::__error() } == 0 {
                    Ok(names)
                } else {
                    Err(ledger_io())
                };
            }
            let bytes = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
            if matches!(bytes, b"." | b"..") {
                continue;
            }
            if names.len() >= MAX_STORE_ENTRIES {
                break Err(tampered());
            }
            let name = std::str::from_utf8(bytes).map_err(|_| tampered())?;
            if !matches!(name, KEY_NAME | LOCK_NAME | STATE_NAME) {
                break Err(tampered());
            }
            if !names.insert(name.to_owned()) {
                break Err(tampered());
            }
        };
        if unsafe { libc::closedir(stream) } != 0 {
            return Err(ledger_io());
        }
        self.verify_root()?;
        result
    }

    pub(crate) fn is_empty_unclassified(&self) -> Result<bool, LedgerError> {
        self.verify_root()?;
        let descriptor = unsafe {
            libc::openat(
                self.directory.as_raw_fd(),
                c".".as_ptr(),
                libc::O_RDONLY
                    | libc::O_DIRECTORY
                    | libc::O_CLOEXEC
                    | libc::O_NOFOLLOW
                    | libc::O_NONBLOCK,
            )
        };
        if descriptor < 0 {
            return Err(ledger_io());
        }
        let stream = unsafe { libc::fdopendir(descriptor) };
        if stream.is_null() {
            unsafe { libc::close(descriptor) };
            return Err(ledger_io());
        }
        let result = loop {
            unsafe { *libc::__error() = 0 };
            let entry = unsafe { libc::readdir(stream) };
            if entry.is_null() {
                break if unsafe { *libc::__error() } == 0 {
                    Ok(true)
                } else {
                    Err(ledger_io())
                };
            }
            let bytes = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
            if !matches!(bytes, b"." | b"..") {
                break Ok(false);
            }
        };
        if unsafe { libc::closedir(stream) } != 0 {
            return Err(ledger_io());
        }
        self.verify_root()?;
        result
    }
}
