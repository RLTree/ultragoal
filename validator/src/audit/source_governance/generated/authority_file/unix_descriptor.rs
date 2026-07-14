use super::descriptor_path::DescriptorPath;
use super::identity::{AuthorityFileBaseline, NodeIdentity, NodeKind};
use super::{AuthorityFileCapture, AuthorityFileReadError, AuthorityFileReadErrorId, error};
use std::ffi::{CString, OsString};
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

pub(super) struct RootHandle {
    descriptor: File,
    identity: NodeIdentity,
}

pub(super) fn open_root(path: &Path) -> Result<RootHandle, AuthorityFileReadError> {
    let parsed = DescriptorPath::root(path)?;
    let mut directory = open_anchor(parsed.absolute)?;
    for component in &parsed.components {
        directory = open_component(&directory, component, directory_flags())
            .map_err(|_| error(AuthorityFileReadErrorId::RootRejected))?;
        require_directory(&directory, AuthorityFileReadErrorId::RootRejected)?;
    }
    let metadata = directory
        .metadata()
        .map_err(|_| error(AuthorityFileReadErrorId::RootRejected))?;
    let identity = node_identity(&metadata)
        .filter(|value| value.kind == NodeKind::Directory)
        .ok_or_else(|| error(AuthorityFileReadErrorId::RootRejected))?;
    Ok(RootHandle {
        descriptor: directory,
        identity,
    })
}

pub(super) fn capture(
    root: &RootHandle,
    relative: &Path,
) -> Result<AuthorityFileCapture, AuthorityFileReadError> {
    let parsed = DescriptorPath::relative(relative)?;
    let mut directories = Vec::new();
    let mut ancestors = Vec::new();
    for component in &parsed.components[..parsed.components.len() - 1] {
        let parent = directories.last().unwrap_or(&root.descriptor);
        let directory = open_component(parent, component, directory_flags())
            .map_err(|_| error(AuthorityFileReadErrorId::AncestorRejected))?;
        let identity = require_directory(&directory, AuthorityFileReadErrorId::AncestorRejected)?;
        directories.push(directory);
        ancestors.push(identity);
    }
    let parent = directories.last().unwrap_or(&root.descriptor);
    let leaf_name = parsed
        .components
        .last()
        .ok_or_else(|| error(AuthorityFileReadErrorId::PathInvalid))?;
    let mut leaf = open_component(parent, leaf_name, file_flags())
        .map_err(|_| error(AuthorityFileReadErrorId::LeafRejected))?;
    let (bytes, leaf_identity) = read_stable_file(&mut leaf)?;
    require_unchanged_directory(&root.descriptor, root.identity)?;
    for (directory, identity) in directories.iter().zip(&ancestors) {
        require_unchanged_directory(directory, *identity)?;
    }
    let baseline = AuthorityFileBaseline::new(
        parsed.relative_path(),
        root.identity,
        ancestors,
        leaf_identity,
        &bytes,
    );
    Ok(AuthorityFileCapture { bytes, baseline })
}

fn open_anchor(absolute: bool) -> Result<File, AuthorityFileReadError> {
    let path = if absolute {
        Path::new("/")
    } else {
        Path::new(".")
    };
    let mut options = OpenOptions::new();
    options.read(true).custom_flags(directory_custom_flags());
    options
        .open(path)
        .map_err(|_| error(AuthorityFileReadErrorId::RootUnavailable))
}

fn open_component(parent: &File, name: &OsString, flags: i32) -> Result<File, ()> {
    let name = CString::new(name.as_bytes()).map_err(|_| ())?;
    // SAFETY: `parent` is a live directory descriptor, `name` is a NUL-terminated
    // single path component, and the caller supplies read-only `openat` flags.
    let descriptor = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if descriptor < 0 {
        Err(())
    } else {
        // SAFETY: a nonnegative `openat` result is a newly owned descriptor and
        // this is its sole conversion into an owning `File`.
        Ok(unsafe { File::from_raw_fd(descriptor) })
    }
}

fn require_directory(
    directory: &File,
    rejected: AuthorityFileReadErrorId,
) -> Result<NodeIdentity, AuthorityFileReadError> {
    directory
        .metadata()
        .ok()
        .and_then(|metadata| node_identity(&metadata))
        .filter(|identity| identity.kind == NodeKind::Directory)
        .ok_or_else(|| error(rejected))
}

fn require_unchanged_directory(
    directory: &File,
    expected: NodeIdentity,
) -> Result<(), AuthorityFileReadError> {
    let current = require_directory(directory, AuthorityFileReadErrorId::ChangedDuringRead)?;
    if current == expected {
        Ok(())
    } else {
        Err(error(AuthorityFileReadErrorId::ChangedDuringRead))
    }
}

fn read_stable_file(file: &mut File) -> Result<(Vec<u8>, NodeIdentity), AuthorityFileReadError> {
    let before = file
        .metadata()
        .map_err(|_| error(AuthorityFileReadErrorId::LeafRejected))?;
    let before_identity = ReadIdentity::new(&before)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|_| error(AuthorityFileReadErrorId::ReadFailed))?;
    let after = file
        .metadata()
        .map_err(|_| error(AuthorityFileReadErrorId::ChangedDuringRead))?;
    if ReadIdentity::new(&after)? != before_identity || after.len() != bytes.len() as u64 {
        return Err(error(AuthorityFileReadErrorId::ChangedDuringRead));
    }
    Ok((bytes, before_identity.node))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ReadIdentity {
    node: NodeIdentity,
    length: u64,
    modified_seconds: i64,
    modified_nanos: i64,
    changed_seconds: i64,
    changed_nanos: i64,
    links: u64,
}

impl ReadIdentity {
    fn new(metadata: &std::fs::Metadata) -> Result<Self, AuthorityFileReadError> {
        let node = node_identity(metadata)
            .filter(|identity| identity.kind == NodeKind::RegularFile)
            .ok_or_else(|| error(AuthorityFileReadErrorId::LeafNotRegular))?;
        if metadata.nlink() != 1 {
            return Err(error(AuthorityFileReadErrorId::LeafLinked));
        }
        Ok(Self {
            node,
            length: metadata.len(),
            modified_seconds: metadata.mtime(),
            modified_nanos: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanos: metadata.ctime_nsec(),
            links: metadata.nlink(),
        })
    }
}

fn node_identity(metadata: &std::fs::Metadata) -> Option<NodeIdentity> {
    let kind = if metadata.is_dir() {
        NodeKind::Directory
    } else if metadata.is_file() {
        NodeKind::RegularFile
    } else {
        return None;
    };
    Some(NodeIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        kind,
        mode: metadata.mode(),
    })
}

fn directory_custom_flags() -> i32 {
    libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK
}

fn directory_flags() -> i32 {
    libc::O_RDONLY | directory_custom_flags()
}

fn file_flags() -> i32 {
    libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK
}
