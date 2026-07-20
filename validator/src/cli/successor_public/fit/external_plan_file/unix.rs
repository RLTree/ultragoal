use std::ffi::CString;
use std::fs::{File, Metadata};
use std::io::Read;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path};

#[derive(Clone, Copy, Eq, PartialEq)]
struct FileIdentity {
    device: u64,
    inode: u64,
    mode: u32,
    links: u64,
    owner: u32,
    length: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

pub(super) fn read_immutable_plan(path: &Path, maximum: u64) -> Result<Vec<u8>, &'static str> {
    let (parent, leaf) = split_absolute(path)?;
    let canonical_parent = parent
        .canonicalize()
        .map_err(|_| "plan-input-parent-open-failed")?;
    if canonical_parent != parent {
        return Err("plan-input-parent-alias");
    }
    let observed_path = parent.join(leaf);
    let directory = open_directory_chain(parent)?;
    let file = open_leaf(&directory, leaf)?;
    let before = identity(&file.metadata().map_err(|_| "plan-input-stat-failed")?);
    let named_before =
        identity(&std::fs::symlink_metadata(&observed_path).map_err(|_| "plan-input-stat-failed")?);
    // SAFETY: geteuid has no preconditions and only reads the process credential.
    if before != named_before
        || before.mode & libc::S_IFMT as u32 != libc::S_IFREG as u32
        || before.links != 1
        || before.owner != unsafe { libc::geteuid() }
        || before.length == 0
        || before.length > maximum
    {
        return Err("plan-input-identity-invalid");
    }
    let bytes = read_exact_length(&file, before.length)?;
    let after = identity(&file.metadata().map_err(|_| "plan-input-stat-failed")?);
    let named_after =
        identity(&std::fs::symlink_metadata(&observed_path).map_err(|_| "plan-input-stat-failed")?);
    (before == after && after == named_after)
        .then_some(bytes)
        .ok_or("plan-input-changed")
}

fn split_absolute(path: &Path) -> Result<(&Path, &std::ffi::OsStr), &'static str> {
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err("plan-input-path-invalid");
    }
    Ok((
        path.parent().ok_or("plan-input-path-invalid")?,
        path.file_name().ok_or("plan-input-path-invalid")?,
    ))
}

fn open_directory_chain(path: &Path) -> Result<File, &'static str> {
    let root = CString::new("/").map_err(|_| "plan-input-path-invalid")?;
    // SAFETY: the fixed C string is NUL-terminated and flags are constants.
    let root = unsafe {
        libc::open(
            root.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if root < 0 {
        return Err("plan-input-parent-open-failed");
    }
    // SAFETY: the successful descriptor is uniquely owned by this File.
    let mut directory = unsafe { File::from_raw_fd(root) };
    for component in path.components() {
        let Component::Normal(component) = component else {
            continue;
        };
        let name = CString::new(component.as_bytes()).map_err(|_| "plan-input-path-invalid")?;
        // SAFETY: the directory descriptor and C string remain valid for the call.
        let descriptor = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if descriptor < 0 {
            return Err("plan-input-parent-open-failed");
        }
        // SAFETY: the successful descriptor is uniquely owned by this File.
        directory = unsafe { File::from_raw_fd(descriptor) };
    }
    Ok(directory)
}

fn open_leaf(directory: &File, leaf: &std::ffi::OsStr) -> Result<File, &'static str> {
    let name = CString::new(leaf.as_bytes()).map_err(|_| "plan-input-path-invalid")?;
    // SAFETY: the directory descriptor and C string remain valid for the call.
    let descriptor = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        )
    };
    if descriptor < 0 {
        return Err("plan-input-open-failed");
    }
    // SAFETY: the successful descriptor is uniquely owned by this File.
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

fn read_exact_length(file: &File, length: u64) -> Result<Vec<u8>, &'static str> {
    let mut reader = file.try_clone().map_err(|_| "plan-input-read-failed")?;
    let mut bytes = Vec::with_capacity(length as usize);
    reader
        .read_to_end(&mut bytes)
        .map_err(|_| "plan-input-read-failed")?;
    (bytes.len() as u64 == length)
        .then_some(bytes)
        .ok_or("plan-input-read-truncated")
}

fn identity(metadata: &Metadata) -> FileIdentity {
    FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        mode: metadata.mode(),
        links: metadata.nlink(),
        owner: metadata.uid(),
        length: metadata.size(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
    }
}
