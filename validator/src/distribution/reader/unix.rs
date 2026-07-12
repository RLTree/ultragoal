use super::{ObjectIdentity, sha256};
use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use std::ffi::CString;
use std::fs::File;
use std::io::Read;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

pub(crate) struct Root(File);

pub(crate) fn open_root(path: &Path) -> Result<Root, DistributionError> {
    let name = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| error(DistributionErrorId::InvalidPath))?;
    let descriptor = unsafe {
        libc::open(
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if descriptor < 0 {
        return Err(open_error());
    }
    let file = unsafe { File::from_raw_fd(descriptor) };
    if !file
        .metadata()
        .map_err(|_| error(DistributionErrorId::UnsafeObject))?
        .is_dir()
    {
        return Err(error(DistributionErrorId::UnsafeObject));
    }
    Ok(Root(file))
}

pub(crate) fn read(
    root: &Root,
    path: &str,
    maximum: usize,
) -> Result<(Vec<u8>, ObjectIdentity), DistributionError> {
    let duplicated = unsafe { libc::fcntl(root.0.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 0) };
    if duplicated < 0 {
        return Err(error(DistributionErrorId::ObjectUnavailable));
    }
    let mut directory = unsafe { File::from_raw_fd(duplicated) };
    let parts = path.split('/').collect::<Vec<_>>();
    for component in &parts[..parts.len() - 1] {
        let name = CString::new(component.as_bytes())
            .map_err(|_| error(DistributionErrorId::InvalidPath))?;
        let descriptor = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            )
        };
        if descriptor < 0 {
            return Err(open_error());
        }
        directory = unsafe { File::from_raw_fd(descriptor) };
    }
    let name = CString::new(parts[parts.len() - 1].as_bytes())
        .map_err(|_| error(DistributionErrorId::InvalidPath))?;
    let descriptor = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
        )
    };
    if descriptor < 0 {
        return Err(open_error());
    }
    let mut file = unsafe { File::from_raw_fd(descriptor) };
    let before = file
        .metadata()
        .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
    if !before.is_file() || before.nlink() != 1 {
        return Err(error(DistributionErrorId::UnsafeObject));
    }
    if before.len() > maximum as u64 {
        return Err(error(DistributionErrorId::ObjectTooLarge));
    }
    let mut bytes = Vec::with_capacity(before.len() as usize);
    file.by_ref()
        .take(maximum as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
    if bytes.len() > maximum {
        return Err(error(DistributionErrorId::ObjectTooLarge));
    }
    let after = file
        .metadata()
        .map_err(|_| error(DistributionErrorId::ObjectChanged))?;
    let before_identity = metadata_identity(&before, &bytes);
    if metadata_identity(&after, &bytes) != before_identity || after.len() != bytes.len() as u64 {
        return Err(error(DistributionErrorId::ObjectChanged));
    }
    Ok((bytes, before_identity))
}

fn metadata_identity(metadata: &std::fs::Metadata, bytes: &[u8]) -> ObjectIdentity {
    ObjectIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        length: metadata.len(),
        modified_seconds: metadata.mtime(),
        modified_nanos: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanos: metadata.ctime_nsec(),
        sha256: sha256(bytes),
    }
}

fn open_error() -> DistributionError {
    match std::io::Error::last_os_error().raw_os_error() {
        Some(libc::ENOENT) => error(DistributionErrorId::ObjectUnavailable),
        Some(libc::EFBIG) => error(DistributionErrorId::ObjectTooLarge),
        _ => error(DistributionErrorId::UnsafeObject),
    }
}
