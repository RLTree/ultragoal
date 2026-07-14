use super::*;

#[cfg(unix)]
pub(crate) fn remove_created_leaf(
    parent: &VerifiedParent,
    created: FileIdentity,
) -> Result<(), String> {
    if inspect_leaf(parent)? != Some(created) {
        return Ok(());
    }
    let result = unsafe { libc::unlinkat(parent.directory().as_raw_fd(), parent.name.as_ptr(), 0) };
    if result == 0 || std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound {
        Ok(())
    } else {
        Err(io_code("rollback", std::io::Error::last_os_error()))
    }
}

#[cfg(unix)]
pub(crate) fn identity(metadata: &Metadata) -> FileIdentity {
    FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    }
}

#[cfg(unix)]
pub(crate) fn validate_metadata(metadata: &Metadata) -> Result<(), String> {
    if metadata.file_type().is_symlink() {
        return Err("observe-store-path-denied: symlink store rejected".to_owned());
    }
    if !metadata.is_file() {
        return Err("observe-store-path-denied: special store rejected".to_owned());
    }
    if metadata.nlink() != 1 {
        return Err("observe-store-path-denied: hardlinked store rejected".to_owned());
    }
    Ok(())
}

pub(crate) fn io_code(operation: &str, error: std::io::Error) -> String {
    let category = match error.kind() {
        std::io::ErrorKind::PermissionDenied => "permission-denied",
        std::io::ErrorKind::NotFound => "not-found",
        std::io::ErrorKind::AlreadyExists => "already-exists",
        std::io::ErrorKind::WouldBlock => "would-block",
        _ => "io-failure",
    };
    format!("observe-store-{operation}:{category}")
}

#[cfg(not(unix))]
pub(crate) fn unsupported_platform() -> String {
    "observe-store-path-denied: secure path traversal unsupported".to_owned()
}

#[cfg(unix)]
pub(crate) fn cstr(bytes: &'static [u8]) -> CString {
    CString::from_vec_with_nul(bytes.to_vec()).expect("literal C string")
}

#[cfg(unix)]
pub(crate) fn open_directory(directory: libc::c_int, name: &CStr) -> Result<File, String> {
    let fd = unsafe {
        libc::openat(
            directory,
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if fd < 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ELOOP) || entry_is_symlink(directory, name) {
            return Err("observe-store-path-denied: ancestor symlink rejected".to_owned());
        }
        return Err(io_code("parent", error));
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}

#[cfg(unix)]
pub(crate) fn entry_is_symlink(directory: libc::c_int, name: &CStr) -> bool {
    let mut stat = unsafe { std::mem::zeroed::<libc::stat>() };
    unsafe {
        libc::fstatat(
            directory,
            name.as_ptr(),
            &mut stat,
            libc::AT_SYMLINK_NOFOLLOW,
        ) == 0
            && (stat.st_mode & libc::S_IFMT) == libc::S_IFLNK
    }
}
