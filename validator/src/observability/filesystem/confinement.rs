use std::fs::{File, Metadata};

#[cfg(unix)]
use std::ffi::{CStr, CString};
#[cfg(unix)]
use std::os::fd::{AsRawFd, FromRawFd};
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(unix)]
use std::path::{Component, Path};

use super::FileIdentity;
#[cfg(unix)]
use super::anchors::{
    absolute_parent, directory_identity, identity_stat, validate_directory_link, validate_stat,
};

#[cfg(unix)]
#[derive(Debug)]
pub(crate) struct VerifiedParent {
    ancestors: Vec<BoundDirectory>,
    name: CString,
}

#[cfg(not(unix))]
#[derive(Debug)]
pub(crate) struct VerifiedParent;

#[cfg(unix)]
#[derive(Debug)]
struct BoundDirectory {
    directory: File,
    identity: FileIdentity,
    name_from_parent: Option<CString>,
}

#[cfg(unix)]
pub(super) fn open_verified_parent(path: &Path) -> Result<VerifiedParent, String> {
    let lexical_parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| "observe-store-path-denied: parent is required".to_owned())?;
    let name = path
        .file_name()
        .ok_or_else(|| "observe-store-path-denied: file name is required".to_owned())?;
    let name = CString::new(name.as_bytes())
        .map_err(|_| "observe-store-path-denied: file name is required".to_owned())?;

    // Open each lexical ancestor through a retained directory descriptor. This
    // deliberately avoids canonicalize: canonicalization would follow the very
    // symlink this boundary is supposed to reject.
    let cwd_binding = if lexical_parent.is_absolute() {
        None
    } else {
        let cwd = open_directory(libc::AT_FDCWD, &cstr(b".\0"))?;
        Some(absolute_parent(lexical_parent, directory_identity(&cwd)?)?)
    };
    let parent = cwd_binding
        .as_ref()
        .map_or_else(|| lexical_parent.to_path_buf(), |binding| binding.0.clone());
    let directory = open_directory(libc::AT_FDCWD, &cstr(b"/\0"))?;
    let mut ancestors = vec![BoundDirectory {
        identity: directory_identity(&directory)?,
        directory,
        name_from_parent: None,
    }];
    let mut normal_depth = 0_usize;
    let mut cwd_attached = cwd_binding
        .as_ref()
        .is_none_or(|binding| binding.1 == 0 && ancestors[0].identity == binding.2);
    for component in parent.components() {
        let component = match component {
            Component::RootDir | Component::CurDir => continue,
            Component::ParentDir => cstr(b"..\0"),
            Component::Normal(item) => {
                normal_depth += 1;
                CString::new(item.as_bytes())
                    .map_err(|_| "observe-store-path-denied: parent is required".to_owned())?
            }
            Component::Prefix(_) => {
                return Err("observe-store-path-denied: parent is required".to_owned());
            }
        };
        let directory = open_directory(
            ancestors
                .last()
                .expect("verified parent always has an anchor")
                .directory
                .as_raw_fd(),
            &component,
        )?;
        ancestors.push(BoundDirectory {
            identity: directory_identity(&directory)?,
            directory,
            name_from_parent: Some(component),
        });
        if cwd_binding
            .as_ref()
            .is_some_and(|binding| normal_depth == binding.1)
        {
            cwd_attached = ancestors
                .last()
                .is_some_and(|item| item.identity == cwd_binding.as_ref().unwrap().2);
        }
    }
    if !cwd_attached {
        return Err(
            "observe-store-path-denied: current directory substitution detected".to_owned(),
        );
    }
    Ok(VerifiedParent { ancestors, name })
}

#[cfg(unix)]
impl VerifiedParent {
    fn directory(&self) -> &File {
        &self
            .ancestors
            .last()
            .expect("verified parent always has an anchor")
            .directory
    }

    pub(super) fn revalidate(&self) -> Result<(), String> {
        for pair in self.ancestors.windows(2) {
            let parent = &pair[0];
            let child = &pair[1];
            validate_directory_link(
                parent.directory.as_raw_fd(),
                child
                    .name_from_parent
                    .as_deref()
                    .expect("non-root anchor has a parent name"),
                child.identity,
            )?;
        }
        Ok(())
    }
}

#[cfg(unix)]
pub(super) fn inspect_leaf(parent: &VerifiedParent) -> Result<Option<FileIdentity>, String> {
    let mut stat = unsafe { std::mem::zeroed::<libc::stat>() };
    let result = unsafe {
        libc::fstatat(
            parent.directory().as_raw_fd(),
            parent.name.as_ptr(),
            &mut stat,
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result == 0 {
        validate_stat(&stat)?;
        return Ok(Some(identity_stat(&stat)));
    }
    let error = std::io::Error::last_os_error();
    if error.kind() == std::io::ErrorKind::NotFound {
        Ok(None)
    } else {
        Err(io_code("metadata", error))
    }
}

#[cfg(unix)]
pub(super) fn open_leaf(
    parent: &VerifiedParent,
    flags: libc::c_int,
    operation: &str,
) -> Result<File, String> {
    let fd = unsafe {
        libc::openat(
            parent.directory().as_raw_fd(),
            parent.name.as_ptr(),
            flags | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
            0o600,
        )
    };
    if fd < 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ELOOP) {
            return Err("observe-store-path-denied: symlink store rejected".to_owned());
        }
        return Err(io_code(operation, error));
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}

#[cfg(unix)]
pub(super) fn validate_file(
    parent: &VerifiedParent,
    file: &File,
    expected: Option<FileIdentity>,
) -> Result<(), String> {
    parent.revalidate()?;
    let opened = file
        .metadata()
        .map_err(|error| io_code("metadata", error))?;
    validate_metadata(&opened)?;
    let current = inspect_leaf(parent)?
        .ok_or_else(|| "observe-store-path-denied: path substitution detected".to_owned())?;
    let opened_identity = identity(&opened);
    if opened_identity != current || expected.is_some_and(|item| item != opened_identity) {
        return Err("observe-store-path-denied: path substitution detected".to_owned());
    }
    Ok(())
}

#[cfg(unix)]
pub(super) fn remove_created_leaf(
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
pub(super) fn identity(metadata: &Metadata) -> FileIdentity {
    FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    }
}

#[cfg(unix)]
pub(super) fn validate_metadata(metadata: &Metadata) -> Result<(), String> {
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

pub(super) fn io_code(operation: &str, error: std::io::Error) -> String {
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
pub(super) fn unsupported_platform() -> String {
    "observe-store-path-denied: secure path traversal unsupported".to_owned()
}

#[cfg(unix)]
fn cstr(bytes: &'static [u8]) -> CString {
    CString::from_vec_with_nul(bytes.to_vec()).expect("literal C string")
}

#[cfg(unix)]
fn open_directory(directory: libc::c_int, name: &CStr) -> Result<File, String> {
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
fn entry_is_symlink(directory: libc::c_int, name: &CStr) -> bool {
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
