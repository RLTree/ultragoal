use super::*;

impl ProcessLock {
    pub(crate) fn acquire(directory: &AnchoredDirectory, name: &str) -> Result<Self, HostFailure> {
        let prior = stat_at(&directory.file, name)?;
        if prior.is_some_and(|identity| !identity.safe_regular() || identity.size != 0) {
            return Err(HostFailure::Invalid);
        }
        let (file, created) = match prior {
            Some(_) => (
                openat(
                    &directory.file,
                    name,
                    libc::O_RDWR | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
                    0,
                )?,
                false,
            ),
            None => (
                openat(
                    &directory.file,
                    name,
                    libc::O_RDWR
                        | libc::O_CREAT
                        | libc::O_EXCL
                        | libc::O_NOFOLLOW
                        | libc::O_CLOEXEC
                        | libc::O_NONBLOCK,
                    0o600,
                )?,
                true,
            ),
        };
        Self::finish(directory, name, prior, file, created)
    }

    pub(crate) fn acquire_existing(
        directory: &AnchoredDirectory,
        name: &str,
    ) -> Result<Self, HostFailure> {
        let prior = stat_at(&directory.file, name)?.ok_or(HostFailure::Invalid)?;
        if !prior.safe_regular() || prior.size != 0 {
            return Err(HostFailure::Invalid);
        }
        let file = openat(
            &directory.file,
            name,
            libc::O_RDWR | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            0,
        )?;
        Self::finish(directory, name, Some(prior), file, false)
    }

    pub(crate) fn finish(
        directory: &AnchoredDirectory,
        name: &str,
        prior: Option<FileIdentity>,
        file: File,
        created: bool,
    ) -> Result<Self, HostFailure> {
        if created && unsafe { libc::fchmod(file.as_raw_fd(), 0o600) } != 0 {
            return Err(HostFailure::Persistence);
        }
        let identity = identity(&file.metadata().map_err(|_| HostFailure::Invalid)?);
        if !identity.safe_regular()
            || identity.size != 0
            || prior.is_some_and(|prior| prior != identity)
            || stat_at(&directory.file, name)? != Some(identity)
            || unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0
        {
            return Err(HostFailure::Invalid);
        }
        if created {
            sync_directory(&directory.file)?;
        }
        Ok(Self { file, identity })
    }

    pub(crate) fn verify(
        &self,
        directory: &AnchoredDirectory,
        name: &str,
    ) -> Result<(), HostFailure> {
        if identity(&self.file.metadata().map_err(|_| HostFailure::Invalid)?) != self.identity
            || stat_at(&directory.file, name)? != Some(self.identity)
        {
            return Err(HostFailure::Invalid);
        }
        Ok(())
    }
}

impl Drop for ProcessLock {
    fn drop(&mut self) {
        unsafe {
            libc::flock(self.file.as_raw_fd(), libc::LOCK_UN);
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FileIdentity {
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) uid: u32,
    pub(crate) mode: u32,
    pub(crate) links: u64,
    pub(crate) size: u64,
    pub(crate) kind: u32,
}

impl FileIdentity {
    pub(crate) fn safe_regular(self) -> bool {
        self.kind == libc::S_IFREG as u32
            && self.uid == unsafe { libc::geteuid() }
            && self.mode == 0o600
            && self.links == 1
            && self.size <= MAX_PENDING_BYTES
    }

    pub(crate) fn same_object(self, other: Self) -> bool {
        self.device == other.device
            && self.inode == other.inode
            && self.uid == other.uid
            && self.mode == other.mode
            && self.links == other.links
            && self.kind == other.kind
    }

    pub(crate) fn same_directory(self, other: Self) -> bool {
        self.device == other.device
            && self.inode == other.inode
            && self.uid == other.uid
            && self.mode == other.mode
            && self.kind == other.kind
            && self.kind == libc::S_IFDIR as u32
    }
}

pub(crate) fn identity(metadata: &fs::Metadata) -> FileIdentity {
    FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        uid: metadata.uid(),
        mode: metadata.mode() & 0o7777,
        links: metadata.nlink(),
        size: metadata.size(),
        kind: metadata.mode() & libc::S_IFMT as u32,
    }
}

pub(crate) fn stat_at(directory: &File, name: &str) -> Result<Option<FileIdentity>, HostFailure> {
    let encoded = CString::new(name).map_err(|_| HostFailure::Invalid)?;
    let mut value = std::mem::MaybeUninit::<libc::stat>::uninit();
    if unsafe {
        libc::fstatat(
            directory.as_raw_fd(),
            encoded.as_ptr(),
            value.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } != 0
    {
        return if std::io::Error::last_os_error().raw_os_error() == Some(libc::ENOENT) {
            Ok(None)
        } else {
            Err(HostFailure::Invalid)
        };
    }
    let value = unsafe { value.assume_init() };
    Ok(Some(FileIdentity {
        device: value.st_dev as u64,
        inode: value.st_ino as u64,
        uid: value.st_uid,
        mode: value.st_mode as u32 & 0o7777,
        links: value.st_nlink as u64,
        size: u64::try_from(value.st_size).map_err(|_| HostFailure::Invalid)?,
        kind: value.st_mode as u32 & libc::S_IFMT as u32,
    }))
}
