use super::types::{
    DirectoryIdentity, EntryKind, EntryMetadata, component, directory_identity, joined, last_errno,
    open_error,
};
use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::filesystem::hooks::{self, EffectPoint};
use std::ffi::CString;
use std::fs::File;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::sync::Arc;

pub(crate) struct Directory {
    anchor: Arc<Anchor>,
    identity: DirectoryIdentity,
    root_device: u64,
    relative: String,
}

/// The only retained filesystem capability for a distribution transaction.
///
/// Descendants are identities plus paths below this descriptor, not retained
/// mutation descriptors. A descendant can therefore never become authority
/// merely because it was opened while it happened to have a confined name.
struct Anchor {
    file: File,
    identity: DirectoryIdentity,
}

impl std::fmt::Debug for Directory {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Directory")
            .field("identity", &self.identity)
            .field("root_device", &self.root_device)
            .field("relative", &self.relative)
            .finish()
    }
}

impl Directory {
    pub(crate) fn open_path(path: &Path) -> Result<Self, DistributionError> {
        let path = CString::new(path.as_os_str().as_bytes())
            .map_err(|_| error(DistributionErrorId::InvalidPath))?;
        let descriptor = unsafe {
            libc::open(
                path.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            )
        };
        if descriptor < 0 {
            return Err(open_error());
        }
        let file = unsafe { File::from_raw_fd(descriptor) };
        let metadata = file
            .metadata()
            .map_err(|_| error(DistributionErrorId::UnsafeObject))?;
        if !metadata.is_dir() {
            return Err(error(DistributionErrorId::UnsafeObject));
        }
        let identity = directory_identity(&metadata);
        Ok(Self {
            anchor: Arc::new(Anchor { file, identity }),
            identity,
            root_device: identity.device,
            relative: String::new(),
        })
    }

    pub(crate) fn duplicate(&self) -> Result<Self, DistributionError> {
        self.current_descriptor()?;
        Ok(Self {
            anchor: Arc::clone(&self.anchor),
            identity: self.identity,
            root_device: self.root_device,
            relative: self.relative.clone(),
        })
    }

    pub(crate) fn identity(&self) -> DirectoryIdentity {
        self.identity
    }

    pub(crate) fn root_device(&self) -> u64 {
        self.root_device
    }

    pub(crate) fn verify_descriptor(&self) -> Result<(), DistributionError> {
        self.current_descriptor().map(|_| ())
    }

    pub(crate) fn open_directory(&self, name: &str) -> Result<Self, DistributionError> {
        let name = component(name)?;
        let text = name
            .to_str()
            .map_err(|_| error(DistributionErrorId::InvalidPath))?;
        let relative = joined(&self.relative, text);
        hooks::before(EffectPoint::OpenDirectory, &relative);
        let parent = self.current_descriptor()?;
        let descriptor = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            )
        };
        if descriptor < 0 {
            return Err(open_error());
        }
        let file = unsafe { File::from_raw_fd(descriptor) };
        let metadata = file
            .metadata()
            .map_err(|_| error(DistributionErrorId::UnsafeObject))?;
        let identity = directory_identity(&metadata);
        if !metadata.is_dir() || identity.device != self.root_device {
            return Err(error(DistributionErrorId::UnsafeObject));
        }
        Ok(Self {
            anchor: Arc::clone(&self.anchor),
            identity,
            root_device: self.root_device,
            relative,
        })
    }

    pub(crate) fn ensure_directory(&self, name: &str) -> Result<Self, DistributionError> {
        match self.stat(name)? {
            Some(row) if row.kind == EntryKind::Directory => return self.open_directory(name),
            Some(_) => return Err(error(DistributionErrorId::UnsafeObject)),
            None => {}
        }
        let name = component(name)?;
        let text = name
            .to_str()
            .map_err(|_| error(DistributionErrorId::InvalidPath))?;
        hooks::before(EffectPoint::Mkdir, &joined(&self.relative, text));
        let parent = self.current_descriptor()?;
        let result = unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) };
        if result != 0 && last_errno() != Some(libc::EEXIST) {
            return Err(error(DistributionErrorId::EffectFailed));
        }
        self.open_directory(text)
    }

    pub(crate) fn create_directory(&self, name: &str) -> Result<Self, DistributionError> {
        let name = component(name)?;
        let text = name
            .to_str()
            .map_err(|_| error(DistributionErrorId::InvalidPath))?;
        hooks::before(EffectPoint::Mkdir, &joined(&self.relative, text));
        let parent = self.current_descriptor()?;
        if unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } != 0 {
            return Err(error(DistributionErrorId::EffectFailed));
        }
        self.open_directory(text)
    }

    pub(crate) fn stat(&self, name: &str) -> Result<Option<EntryMetadata>, DistributionError> {
        let name = component(name)?;
        let parent = self.current_descriptor()?;
        let mut value = std::mem::MaybeUninit::<libc::stat>::uninit();
        let result = unsafe {
            libc::fstatat(
                parent.as_raw_fd(),
                name.as_ptr(),
                value.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if result != 0 {
            return match last_errno() {
                Some(libc::ENOENT) => Ok(None),
                _ => Err(error(DistributionErrorId::ObjectUnavailable)),
            };
        }
        let value = unsafe { value.assume_init() };
        let mode = value.st_mode as libc::mode_t;
        let kind = match mode & libc::S_IFMT {
            libc::S_IFDIR => EntryKind::Directory,
            libc::S_IFREG => EntryKind::Regular,
            _ => EntryKind::Other,
        };
        Ok(Some(EntryMetadata {
            identity: DirectoryIdentity {
                device: value.st_dev as u64,
                inode: value.st_ino as u64,
            },
            kind,
            links: value.st_nlink as u64,
            length: value.st_size.max(0) as u64,
        }))
    }

    pub(crate) fn names(&self) -> Result<Vec<String>, DistributionError> {
        super::directory_names::names(self)
    }

    pub(crate) fn relative(&self) -> &str {
        &self.relative
    }

    /// Resolve this identity from the transaction anchor at the last possible
    /// point before a mutating syscall. Never retain a descendant descriptor
    /// across an effect boundary.
    pub(crate) fn mutation_descriptor(&self) -> Result<File, DistributionError> {
        self.current_descriptor()
    }

    pub(super) fn current_descriptor(&self) -> Result<File, DistributionError> {
        let anchor_metadata = self
            .anchor
            .file
            .metadata()
            .map_err(|_| error(DistributionErrorId::ObjectChanged))?;
        if !anchor_metadata.is_dir()
            || directory_identity(&anchor_metadata) != self.anchor.identity
            || anchor_metadata.dev() != self.root_device
        {
            return Err(error(DistributionErrorId::ObjectChanged));
        }
        let descriptor = unsafe {
            libc::openat(
                self.anchor.file.as_raw_fd(),
                c".".as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            )
        };
        if descriptor < 0 {
            return Err(error(DistributionErrorId::ObjectChanged));
        }
        let mut current = unsafe { File::from_raw_fd(descriptor) };
        for path_component in self.relative.split('/').filter(|value| !value.is_empty()) {
            let component = component(path_component)?;
            let descriptor = unsafe {
                libc::openat(
                    current.as_raw_fd(),
                    component.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
                )
            };
            if descriptor < 0 {
                return Err(error(DistributionErrorId::ObjectChanged));
            }
            current = unsafe { File::from_raw_fd(descriptor) };
        }
        let metadata = current
            .metadata()
            .map_err(|_| error(DistributionErrorId::ObjectChanged))?;
        if !metadata.is_dir()
            || directory_identity(&metadata) != self.identity
            || metadata.dev() != self.root_device
        {
            return Err(error(DistributionErrorId::ObjectChanged));
        }
        Ok(current)
    }
}
