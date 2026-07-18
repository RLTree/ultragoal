use super::*;

impl Store {
    pub(crate) fn create_exclusive(&self, name: &str, mode: u32) -> Result<File, LedgerError> {
        validate_name(name)?;
        let name = CString::new(name).map_err(|_| invalid_store())?;
        let descriptor = unsafe {
            libc::openat(
                self.directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDWR | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC | libc::O_NOFOLLOW,
                mode,
            )
        };
        if descriptor < 0 {
            return Err(match std::io::Error::last_os_error().raw_os_error() {
                Some(libc::EEXIST) => replay_error(),
                _ => ledger_io(),
            });
        }
        Ok(unsafe { File::from_raw_fd(descriptor) })
    }

    pub(crate) fn open_existing(&self, name: &str, flags: i32) -> Result<File, LedgerError> {
        validate_name(name)?;
        let name = CString::new(name).map_err(|_| invalid_store())?;
        let descriptor = unsafe {
            libc::openat(
                self.directory.as_raw_fd(),
                name.as_ptr(),
                flags | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
            )
        };
        if descriptor < 0 {
            return Err(tampered());
        }
        Ok(unsafe { File::from_raw_fd(descriptor) })
    }

    pub(crate) fn exact_stat(&self, name: &str) -> Result<Option<FileIdentity>, LedgerError> {
        let name = CString::new(name).map_err(|_| invalid_store())?;
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        let result = unsafe {
            libc::fstatat(
                self.directory.as_raw_fd(),
                name.as_ptr(),
                stat.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if result != 0 {
            return match std::io::Error::last_os_error().raw_os_error() {
                Some(libc::ENOENT) => Ok(None),
                _ => Err(ledger_io()),
            };
        }
        let stat = unsafe { stat.assume_init() };
        Ok(Some(stat_identity(&stat)))
    }
}
