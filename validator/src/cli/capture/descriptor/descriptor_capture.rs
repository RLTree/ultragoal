use super::*;

pub(crate) const MAX_REGULAR_BYTES: usize = 64 * 1024 * 1024;

#[cfg(test)]
thread_local! {
    static TEST_DESCRIPTOR_BYTES_READ: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static TEST_FILE_OPEN_ATTEMPTS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn reset_test_descriptor_bytes_read() {
    TEST_DESCRIPTOR_BYTES_READ.set(0);
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn test_descriptor_bytes_read() -> u64 {
    TEST_DESCRIPTOR_BYTES_READ.get()
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn reset_test_file_open_attempts() {
    TEST_FILE_OPEN_ATTEMPTS.set(0);
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn test_file_open_attempts() -> u64 {
    TEST_FILE_OPEN_ATTEMPTS.get()
}

#[cfg(test)]
pub(crate) fn record_test_descriptor_bytes_read(bytes: usize) {
    TEST_DESCRIPTOR_BYTES_READ.with(|total| {
        total.set(total.get().saturating_add(bytes as u64));
    });
}

#[cfg(test)]
pub(crate) fn record_test_file_open_attempt() {
    TEST_FILE_OPEN_ATTEMPTS.with(|total| total.set(total.get().saturating_add(1)));
}

#[cfg(not(test))]
pub(crate) fn record_test_file_open_attempt() {}

#[cfg(not(test))]
pub(crate) fn record_test_descriptor_bytes_read(_bytes: usize) {}

#[cfg(unix)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Snapshot {
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) links: u64,
    pub(crate) mode: u32,
    pub(crate) length: u64,
    pub(crate) modified_seconds: i64,
    pub(crate) modified_nanos: i64,
    pub(crate) changed_seconds: i64,
    pub(crate) changed_nanos: i64,
}

#[cfg(unix)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DirectoryIdentity {
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) mode: u32,
}

#[cfg(unix)]
impl DirectoryIdentity {
    pub fn from(metadata: &Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            mode: metadata.mode(),
        }
    }
}

#[cfg(unix)]
impl Snapshot {
    pub fn from(metadata: &Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            links: metadata.nlink(),
            mode: metadata.mode(),
            length: metadata.len(),
            modified_seconds: metadata.mtime(),
            modified_nanos: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanos: metadata.ctime_nsec(),
        }
    }
}

#[cfg(unix)]
pub(crate) fn c_name(name: &std::ffi::OsStr) -> Result<CString, String> {
    CString::new(name.as_bytes()).map_err(|_| "path component contains NUL".to_owned())
}

#[cfg(unix)]
pub(crate) fn open_dir_at(directory: &File, name: &std::ffi::OsStr) -> Result<File, String> {
    let name = c_name(name)?;
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err("confined directory open failed".to_owned());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}

#[cfg(not(unix))]
pub(crate) fn open_dir_at(_directory: &File, _name: &std::ffi::OsStr) -> Result<File, String> {
    Err("descriptor-confined capture requires Unix".to_owned())
}

#[cfg(unix)]
pub(crate) fn open_file_at(
    directory: &File,
    name: &std::ffi::OsStr,
    _executable: bool,
) -> Result<File, String> {
    let name = c_name(name)?;
    let expected = regular_identity_at(directory, &name)?;
    #[cfg(test)]
    super::super::descriptor_race_control::pause_before_file_open();
    record_test_file_open_attempt();
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err("confined regular-file open failed".to_owned());
    }
    let file = unsafe { File::from_raw_fd(fd) };
    let metadata = file
        .metadata()
        .map_err(|_| "confined regular-file metadata failed".to_owned())?;
    if (metadata.dev(), metadata.ino()) != expected || !metadata.is_file() {
        return Err("confined regular-file identity changed before capture".to_owned());
    }
    Ok(file)
}

#[cfg(unix)]
pub(crate) fn regular_identity_at(directory: &File, name: &CString) -> Result<(u64, u64), String> {
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    let status = unsafe {
        libc::fstatat(
            directory.as_raw_fd(),
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if status != 0 {
        return Err("confined regular-file open failed".to_owned());
    }
    let stat = unsafe { stat.assume_init() };
    if !regular_mode(stat.st_mode) {
        return Err("confined regular-file open failed".to_owned());
    }
    Ok((stat.st_dev as u64, stat.st_ino as u64))
}

#[cfg(unix)]
pub(crate) fn regular_mode(mode: libc::mode_t) -> bool {
    mode & libc::S_IFMT == libc::S_IFREG
}

#[cfg(not(unix))]
pub(crate) fn open_file_at(
    _directory: &File,
    _name: &std::ffi::OsStr,
    _executable: bool,
) -> Result<File, String> {
    Err("descriptor-confined capture requires Unix".to_owned())
}

#[cfg(unix)]
pub(crate) fn read_descriptor(
    file: &File,
    maximum_bytes: usize,
) -> Result<(Vec<u8>, String), String> {
    let metadata = file
        .metadata()
        .map_err(|_| "regular-file metadata failed".to_owned())?;
    let maximum_bytes = maximum_bytes.min(MAX_REGULAR_BYTES);
    if metadata.len() > maximum_bytes as u64 {
        return Err("regular file exceeds configured read bound before read".to_owned());
    }
    let mut bytes = vec![0_u8; metadata.len() as usize];
    let mut offset = 0_usize;
    while offset < bytes.len() {
        let read = file
            .read_at(&mut bytes[offset..], offset as u64)
            .map_err(|_| "descriptor read failed".to_owned())?;
        if read == 0 {
            return Err("regular file shortened during read".to_owned());
        }
        record_test_descriptor_bytes_read(read);
        offset += read;
    }
    let after = file
        .metadata()
        .map_err(|_| "regular-file metadata recheck failed".to_owned())?;
    if Snapshot::from(&metadata) != Snapshot::from(&after) {
        return Err("regular file changed during read".to_owned());
    }
    let sha256 = digest_bytes(&bytes);
    Ok((bytes, sha256))
}
