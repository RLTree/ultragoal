use std::ffi::CString;
use std::io::Read;
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, FromRawFd};

impl ProcessLock {
    fn acquire(file: File) -> Result<Self, ProductError> {
        // SAFETY: `file` owns a live descriptor for the exact validated lock
        // file, and `flock` neither retains a pointer nor crosses ownership.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } != 0 {
            return Err(ProductError::AuthorityStoreInvalid);
        }
        Ok(Self(file))
    }
}

impl Drop for ProcessLock {
    fn drop(&mut self) {
        // SAFETY: the owned file remains alive throughout `drop`; unlocking a
        // descriptor that this guard locked is valid and has no pointer alias.
        let _ = unsafe { libc::flock(self.0.as_raw_fd(), libc::LOCK_UN) };
    }
}

fn effective_uid() -> u32 {
    // SAFETY: `geteuid` takes no arguments, returns process-local identity,
    // and does not expose or retain memory.
    unsafe { libc::geteuid() }
}

fn open_directory(root: &Path) -> Result<File, ProductError> {
    validate_root(root)?;
    let directory = File::open(root).map_err(|_| ProductError::AuthorityStoreInvalid)?;
    verify_directory_path(root, &directory)?;
    Ok(directory)
}

fn verify_directory_path(root: &Path, directory: &File) -> Result<(), ProductError> {
    validate_root(root)?;
    let path = fs::symlink_metadata(root).map_err(|_| ProductError::AuthorityStoreInvalid)?;
    let opened = directory
        .metadata()
        .map_err(|_| ProductError::AuthorityStoreInvalid)?;
    if path.dev() != opened.dev()
        || path.ino() != opened.ino()
        || path.uid() != opened.uid()
        || path.mode() != opened.mode()
    {
        return Err(ProductError::AuthorityStoreInvalid);
    }
    Ok(())
}

fn entry_exists(directory: &File, name: &str) -> Result<bool, ProductError> {
    let name = CString::new(name).map_err(|_| ProductError::AuthorityStoreInvalid)?;
    let mut stat = MaybeUninit::<libc::stat>::uninit();
    // SAFETY: both descriptors and the C string remain live for the call;
    // `stat` points to initialized writable storage of the required type.
    let result = unsafe {
        libc::fstatat(
            directory.as_raw_fd(),
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result == 0 {
        return Ok(true);
    }
    match std::io::Error::last_os_error().raw_os_error() {
        Some(libc::ENOENT) => Ok(false),
        _ => Err(ProductError::AuthorityStoreInvalid),
    }
}

fn open_file_at(
    directory: &File,
    name: &str,
    flags: i32,
    max_bytes: u64,
) -> Result<File, ProductError> {
    let name = CString::new(name).map_err(|_| ProductError::AuthorityStoreInvalid)?;
    // SAFETY: the directory descriptor and C string remain live for the call.
    // On success the returned descriptor is uniquely transferred into `File`.
    let descriptor = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            flags | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            0o600,
        )
    };
    if descriptor < 0 {
        return Err(ProductError::AuthorityStoreInvalid);
    }
    // SAFETY: `openat` returned a new owned descriptor not held elsewhere.
    let file = unsafe { File::from_raw_fd(descriptor) };
    validate_opened_file(&file, max_bytes)?;
    Ok(file)
}

fn validate_opened_file(file: &File, max_bytes: u64) -> Result<(), ProductError> {
    let metadata = file
        .metadata()
        .map_err(|_| ProductError::AuthorityStoreInvalid)?;
    if !metadata.is_file()
        || metadata.uid() != effective_uid()
        || metadata.mode() & 0o7777 != 0o600
        || metadata.nlink() != 1
        || metadata.len() > max_bytes
    {
        return Err(ProductError::AuthorityStoreInvalid);
    }
    Ok(())
}

fn file_identity(file: &File) -> Result<(u64, u64), ProductError> {
    let metadata = file
        .metadata()
        .map_err(|_| ProductError::AuthorityStoreInvalid)?;
    Ok((metadata.dev(), metadata.ino()))
}

fn append_frame(file: &File, bytes: &[u8]) -> Result<(), ProductError> {
    let mut frame = Vec::with_capacity(bytes.len() + 1);
    frame.extend_from_slice(bytes);
    frame.push(b'\n');
    // SAFETY: the descriptor is live and opened for append; the byte slice is
    // valid for exactly `frame.len()` bytes and is not retained by `write`.
    let written = unsafe { libc::write(file.as_raw_fd(), frame.as_ptr().cast(), frame.len()) };
    if written != frame.len() as isize {
        return Err(ProductError::AuthorityStoreInvalid);
    }
    file.sync_data()
        .map_err(|_| ProductError::AuthorityStoreInvalid)
}

fn create_file_at(directory: &File, name: &str, bytes: &[u8]) -> Result<(), ProductError> {
    let mut file = open_file_at(
        directory,
        name,
        libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
        bytes.len() as u64,
    )?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| ProductError::AuthorityStoreInvalid)
}

fn read_bounded_at(directory: &File, name: &str, max_bytes: u64) -> Result<Vec<u8>, ProductError> {
    let file = open_file_at(directory, name, libc::O_RDONLY, max_bytes)?;
    read_bounded_file(file, max_bytes)
}

fn read_bounded_file(mut file: File, max_bytes: u64) -> Result<Vec<u8>, ProductError> {
    let length = file
        .metadata()
        .map_err(|_| ProductError::AuthorityStoreInvalid)?
        .len();
    let mut bytes = Vec::with_capacity(length as usize);
    Read::by_ref(&mut file)
        .take(max_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ProductError::AuthorityStoreInvalid)?;
    validate_opened_file(&file, max_bytes)?;
    if bytes.len() as u64 != length || bytes.len() as u64 > max_bytes {
        return Err(ProductError::AuthorityStoreInvalid);
    }
    Ok(bytes)
}

fn unlink_file_at(directory: &File, name: &str) -> Result<(), ProductError> {
    let name = CString::new(name).map_err(|_| ProductError::AuthorityStoreInvalid)?;
    // SAFETY: the directory descriptor and C string remain valid for the call;
    // flags zero requests unlinking only a non-directory entry.
    if unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) } != 0 {
        return Err(ProductError::AuthorityStoreInvalid);
    }
    Ok(())
}
