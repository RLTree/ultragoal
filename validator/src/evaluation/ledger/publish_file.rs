pub(super) fn publish_file(
    root: &File,
    destination: &str,
    value: &impl Serialize,
    generation: u64,
) -> Result<(), EvaluationLedgerError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-serialization-failed"))?;
    let temporary = format!(
        ".{destination}.pending.{}.{}",
        std::process::id(),
        generation
    );
    let descriptor = openat(
        root,
        &temporary,
        libc::O_WRONLY
            | libc::O_CREAT
            | libc::O_EXCL
            | libc::O_NOFOLLOW
            | libc::O_CLOEXEC
            | libc::O_NONBLOCK,
        0o600,
    )?;
    let mut file = unsafe { File::from_raw_fd(descriptor) };
    file.write_all(&bytes)
        .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-write-failed"))?;
    file.sync_all()
        .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-fsync-failed"))?;
    renameat(root, &temporary, destination)?;
    Ok(())
}

pub(super) fn entry_exists(root: &File, name: &str) -> Result<bool, EvaluationLedgerError> {
    let name = c_string(name)?;
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            root.as_raw_fd(),
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result == 0 {
        return Ok(true);
    }
    if std::io::Error::last_os_error().raw_os_error() == Some(libc::ENOENT) {
        return Ok(false);
    }
    Err(EvaluationLedgerError::new(
        "evaluation-ledger-entry-probe-failed",
    ))
}

pub(super) fn openat(
    root: &File,
    name: &str,
    flags: i32,
    mode: u32,
) -> Result<i32, EvaluationLedgerError> {
    let name = c_string(name)?;
    let descriptor = unsafe { libc::openat(root.as_raw_fd(), name.as_ptr(), flags, mode) };
    if descriptor < 0 {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-openat-failed",
        ));
    }
    Ok(descriptor)
}

fn renameat(root: &File, source: &str, destination: &str) -> Result<(), EvaluationLedgerError> {
    let source = c_string(source)?;
    let destination = c_string(destination)?;
    if unsafe {
        libc::renameat(
            root.as_raw_fd(),
            source.as_ptr(),
            root.as_raw_fd(),
            destination.as_ptr(),
        )
    } != 0
    {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-rename-failed",
        ));
    }
    Ok(())
}

pub(super) fn sync_directory(root: &File) -> Result<(), EvaluationLedgerError> {
    root.sync_all()
        .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-directory-fsync-failed"))
}

fn c_string(value: &str) -> Result<CString, EvaluationLedgerError> {
    CString::new(value)
        .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-component-invalid"))
}

pub(super) fn hmac(bytes: &[u8], key: &[u8; 32]) -> Result<String, EvaluationLedgerError> {
    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-key-invalid"))?;
    mac.update(bytes);
    Ok(format!("sha256:{:x}", mac.finalize().into_bytes()))
}

pub(super) fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub(super) struct FileLock<'a> {
    file: &'a File,
}

impl<'a> FileLock<'a> {
    pub(super) fn exclusive(file: &'a File) -> Result<Self, EvaluationLedgerError> {
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } != 0 {
            return Err(EvaluationLedgerError::new("evaluation-ledger-lock-failed"));
        }
        Ok(Self { file })
    }
}

impl Drop for FileLock<'_> {
    fn drop(&mut self) {
        unsafe {
            libc::flock(self.file.as_raw_fd(), libc::LOCK_UN);
        }
    }
}
