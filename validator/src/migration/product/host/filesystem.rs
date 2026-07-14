use super::{HostError, MAX_HOST_FILE_BYTES};
use std::ffi::CString;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct FileIdentity {
    pub(super) device: u64,
    pub(super) inode: u64,
    pub(super) uid: u32,
    pub(super) mode: u32,
    pub(super) links: u64,
    pub(super) size: u64,
    pub(super) kind: u32,
}

impl FileIdentity {
    pub(super) fn safe_regular(self, exact_owner_only: bool, max_bytes: u64) -> bool {
        self.kind == libc::S_IFREG as u32
            && self.uid == unsafe { libc::geteuid() }
            && self.links == 1
            && self.size <= max_bytes
            && self.mode & 0o022 == 0
            && (!exact_owner_only || self.mode == 0o600)
    }

    fn same_directory(self, other: Self) -> bool {
        self.device == other.device
            && self.inode == other.inode
            && self.uid == other.uid
            && self.mode == other.mode
            && self.kind == other.kind
            && self.kind == libc::S_IFDIR as u32
    }
}

#[derive(Debug)]
pub(super) struct AnchoredDirectory {
    path: PathBuf,
    file: File,
    identity: FileIdentity,
    owner_only: bool,
}

impl AnchoredDirectory {
    pub(super) fn open_absolute(path: &Path, owner_only: bool) -> Result<Self, HostError> {
        if !path.is_absolute() {
            return Err(HostError::new("migration-host-root-not-absolute"));
        }
        let encoded = CString::new(path.as_os_str().as_bytes())
            .map_err(|_| HostError::new("migration-host-root-invalid"))?;
        let descriptor = unsafe {
            libc::open(
                encoded.as_ptr(),
                libc::O_RDONLY
                    | libc::O_DIRECTORY
                    | libc::O_NOFOLLOW
                    | libc::O_CLOEXEC
                    | libc::O_NONBLOCK,
            )
        };
        if descriptor < 0 {
            return Err(HostError::new("migration-host-root-unavailable"));
        }
        let file = unsafe { File::from_raw_fd(descriptor) };
        let canonical =
            fs::canonicalize(path).map_err(|_| HostError::new("migration-host-root-invalid"))?;
        if canonical != path {
            return Err(HostError::new("migration-host-root-not-canonical"));
        }
        let metadata = file
            .metadata()
            .map_err(|_| HostError::new("migration-host-root-invalid"))?;
        let identity = identity(&metadata);
        if !metadata.is_dir()
            || identity.uid != unsafe { libc::geteuid() }
            || identity.mode & 0o022 != 0
            || (owner_only && identity.mode != 0o700)
        {
            return Err(HostError::new("migration-host-root-permission-refused"));
        }
        let value = Self {
            path: path.to_path_buf(),
            file,
            identity,
            owner_only,
        };
        value.verify()?;
        Ok(value)
    }

    pub(super) fn verify(&self) -> Result<(), HostError> {
        let metadata = self
            .file
            .metadata()
            .map_err(|_| HostError::new("migration-host-root-drift"))?;
        let current = identity(&metadata);
        let reopened = Self::open_identity(&self.path)?;
        if !current.same_directory(self.identity)
            || !reopened.same_directory(self.identity)
            || current.uid != unsafe { libc::geteuid() }
            || current.mode & 0o022 != 0
            || (self.owner_only && current.mode != 0o700)
            || fs::canonicalize(&self.path)
                .map(|path| path != self.path)
                .unwrap_or(true)
        {
            return Err(HostError::new("migration-host-root-drift"));
        }
        Ok(())
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn identity(&self) -> FileIdentity {
        self.identity
    }

    pub(super) fn file(&self) -> &File {
        &self.file
    }

    pub(super) fn read_regular(
        &self,
        relative: &str,
        owner_only: bool,
        max_bytes: u64,
    ) -> Result<ObservedFile, HostError> {
        self.verify()?;
        let (parent, name) = self.open_parent(relative)?;
        let before = stat_at(&parent, &name)?
            .ok_or_else(|| HostError::new("migration-host-source-missing"))?;
        if !before.safe_regular(owner_only, max_bytes) {
            return Err(HostError::new("migration-host-source-file-refused"));
        }
        let file = openat(
            &parent,
            &name,
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            0,
        )?;
        let opened = identity(
            &file
                .metadata()
                .map_err(|_| HostError::new("migration-host-source-file-refused"))?,
        );
        if opened != before || !opened.safe_regular(owner_only, max_bytes) {
            return Err(HostError::new("migration-host-source-file-substituted"));
        }
        let mut bytes = Vec::new();
        file.take(max_bytes.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|_| HostError::new("migration-host-source-read-failed"))?;
        if bytes.is_empty() || bytes.len() as u64 > max_bytes {
            return Err(HostError::new("migration-host-source-size-refused"));
        }
        if stat_at(&parent, &name)? != Some(opened) {
            return Err(HostError::new("migration-host-source-file-substituted"));
        }
        self.verify()?;
        Ok(ObservedFile {
            bytes,
            identity: opened,
        })
    }

    pub(super) fn verify_fixed_file(
        &self,
        name: &str,
        expected: FileIdentity,
        expected_bytes: &[u8],
        max_bytes: u64,
    ) -> Result<(), HostError> {
        let observed = self.read_regular(name, true, max_bytes)?;
        if observed.identity != expected || observed.bytes != expected_bytes {
            return Err(HostError::new("migration-host-state-substituted"));
        }
        Ok(())
    }

    pub(super) fn fixed_file_identity(
        &self,
        name: &str,
        max_bytes: u64,
    ) -> Result<FileIdentity, HostError> {
        let identity = stat_at(&self.file, name)?
            .ok_or_else(|| HostError::new("migration-host-state-missing"))?;
        if !identity.safe_regular(true, max_bytes) {
            return Err(HostError::new("migration-host-state-file-refused"));
        }
        Ok(identity)
    }

    pub(super) fn write_atomic(&self, name: &str, bytes: &[u8]) -> Result<(), HostError> {
        self.verify()?;
        if bytes.is_empty() || bytes.len() as u64 > MAX_HOST_FILE_BYTES {
            return Err(HostError::new("migration-host-state-size-refused"));
        }
        let current = stat_at(&self.file, name)?
            .ok_or_else(|| HostError::new("migration-host-state-missing"))?;
        if !current.safe_regular(true, MAX_HOST_FILE_BYTES) {
            return Err(HostError::new("migration-host-state-file-refused"));
        }
        let mut nonce = [0_u8; 16];
        getrandom::fill(&mut nonce)
            .map_err(|_| HostError::new("migration-host-random-unavailable"))?;
        let temporary = format!(".state.tmp-{}", encode_hex(&nonce));
        let mut file = openat(
            &self.file,
            &temporary,
            libc::O_WRONLY
                | libc::O_CREAT
                | libc::O_EXCL
                | libc::O_NOFOLLOW
                | libc::O_CLOEXEC
                | libc::O_NONBLOCK,
            0o600,
        )?;
        let result = (|| {
            if unsafe { libc::fchmod(file.as_raw_fd(), 0o600) } != 0 {
                return Err(HostError::new("migration-host-state-persist-failed"));
            }
            let created = identity(
                &file
                    .metadata()
                    .map_err(|_| HostError::new("migration-host-state-persist-failed"))?,
            );
            if !created.safe_regular(true, MAX_HOST_FILE_BYTES) || created.size != 0 {
                return Err(HostError::new("migration-host-state-temp-refused"));
            }
            file.write_all(bytes)
                .map_err(|_| HostError::new("migration-host-state-persist-failed"))?;
            file.sync_all()
                .map_err(|_| HostError::new("migration-host-state-persist-failed"))?;
            let written = identity(
                &file
                    .metadata()
                    .map_err(|_| HostError::new("migration-host-state-persist-failed"))?,
            );
            if written.device != created.device
                || written.inode != created.inode
                || written.size != bytes.len() as u64
                || stat_at(&self.file, &temporary)? != Some(written)
                || stat_at(&self.file, name)? != Some(current)
            {
                return Err(HostError::new("migration-host-state-temp-substituted"));
            }
            rename_replace(&self.file, &temporary, name)?;
            sync_directory(&self.file)?;
            let published = stat_at(&self.file, name)?
                .ok_or_else(|| HostError::new("migration-host-state-persist-failed"))?;
            if published != written || !published.safe_regular(true, MAX_HOST_FILE_BYTES) {
                return Err(HostError::new("migration-host-state-publish-substituted"));
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = unlink_at(&self.file, &temporary);
            let _ = sync_directory(&self.file);
        }
        self.verify()?;
        result
    }

    fn open_parent(&self, relative: &str) -> Result<(File, String), HostError> {
        let path = Path::new(relative);
        if !relative.is_ascii() || path.is_absolute() {
            return Err(HostError::new("migration-host-source-path-refused"));
        }
        let mut parts = Vec::new();
        for component in path.components() {
            match component {
                Component::Normal(value) => {
                    let value = value
                        .to_str()
                        .ok_or_else(|| HostError::new("migration-host-source-path-refused"))?;
                    if value.is_empty() || value == "." || value == ".." {
                        return Err(HostError::new("migration-host-source-path-refused"));
                    }
                    parts.push(value.to_owned());
                }
                _ => return Err(HostError::new("migration-host-source-path-refused")),
            }
        }
        let name = parts
            .pop()
            .ok_or_else(|| HostError::new("migration-host-source-path-refused"))?;
        let mut current = duplicate(&self.file)?;
        for part in parts {
            current = openat(
                &current,
                &part,
                libc::O_RDONLY
                    | libc::O_DIRECTORY
                    | libc::O_NOFOLLOW
                    | libc::O_CLOEXEC
                    | libc::O_NONBLOCK,
                0,
            )?;
            let metadata = current
                .metadata()
                .map_err(|_| HostError::new("migration-host-source-directory-refused"))?;
            let identity = identity(&metadata);
            if !metadata.is_dir()
                || identity.uid != unsafe { libc::geteuid() }
                || identity.mode & 0o022 != 0
            {
                return Err(HostError::new("migration-host-source-directory-refused"));
            }
        }
        Ok((current, name))
    }

    fn open_identity(path: &Path) -> Result<FileIdentity, HostError> {
        let encoded = CString::new(path.as_os_str().as_bytes())
            .map_err(|_| HostError::new("migration-host-root-invalid"))?;
        let descriptor = unsafe {
            libc::open(
                encoded.as_ptr(),
                libc::O_RDONLY
                    | libc::O_DIRECTORY
                    | libc::O_NOFOLLOW
                    | libc::O_CLOEXEC
                    | libc::O_NONBLOCK,
            )
        };
        if descriptor < 0 {
            return Err(HostError::new("migration-host-root-drift"));
        }
        let file = unsafe { File::from_raw_fd(descriptor) };
        file.metadata()
            .map(|metadata| identity(&metadata))
            .map_err(|_| HostError::new("migration-host-root-drift"))
    }
}

pub(super) struct ObservedFile {
    pub(super) bytes: Vec<u8>,
    pub(super) identity: FileIdentity,
}

pub(super) struct ProcessLock {
    file: File,
    identity: FileIdentity,
}

impl ProcessLock {
    pub(super) fn acquire(directory: &AnchoredDirectory, name: &str) -> Result<Self, HostError> {
        directory.verify()?;
        let expected_identity = directory.fixed_file_identity(name, 0)?;
        if expected_identity.size != 0 {
            return Err(HostError::new("migration-host-lock-file-refused"));
        }
        let file = openat(
            directory.file(),
            name,
            libc::O_RDWR | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            0,
        )?;
        let opened = identity(
            &file
                .metadata()
                .map_err(|_| HostError::new("migration-host-lock-file-refused"))?,
        );
        if opened != expected_identity
            || unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0
        {
            return Err(HostError::new("migration-host-lock-busy"));
        }
        Ok(Self {
            file,
            identity: opened,
        })
    }

    pub(super) fn verify(
        &self,
        directory: &AnchoredDirectory,
        name: &str,
    ) -> Result<(), HostError> {
        let current = identity(
            &self
                .file
                .metadata()
                .map_err(|_| HostError::new("migration-host-lock-substituted"))?,
        );
        if current != self.identity
            || stat_at(directory.file(), name)? != Some(self.identity)
            || !current.safe_regular(true, 0)
        {
            return Err(HostError::new("migration-host-lock-substituted"));
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

pub(super) fn identity(metadata: &fs::Metadata) -> FileIdentity {
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

fn duplicate(file: &File) -> Result<File, HostError> {
    let descriptor = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 0) };
    if descriptor < 0 {
        return Err(HostError::new("migration-host-descriptor-duplicate-failed"));
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

fn openat(
    directory: &File,
    name: &str,
    flags: libc::c_int,
    mode: libc::mode_t,
) -> Result<File, HostError> {
    let encoded = CString::new(name).map_err(|_| HostError::new("migration-host-name-refused"))?;
    let descriptor = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            encoded.as_ptr(),
            flags,
            mode as libc::c_uint,
        )
    };
    if descriptor < 0 {
        return Err(HostError::new("migration-host-openat-failed"));
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

fn stat_at(directory: &File, name: &str) -> Result<Option<FileIdentity>, HostError> {
    let encoded = CString::new(name).map_err(|_| HostError::new("migration-host-name-refused"))?;
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
            Err(HostError::new("migration-host-stat-failed"))
        };
    }
    let value = unsafe { value.assume_init() };
    Ok(Some(FileIdentity {
        device: value.st_dev as u64,
        inode: value.st_ino as u64,
        uid: value.st_uid,
        mode: value.st_mode as u32 & 0o7777,
        links: value.st_nlink as u64,
        size: u64::try_from(value.st_size)
            .map_err(|_| HostError::new("migration-host-stat-failed"))?,
        kind: value.st_mode as u32 & libc::S_IFMT as u32,
    }))
}

fn rename_replace(directory: &File, old: &str, new: &str) -> Result<(), HostError> {
    let old = CString::new(old).map_err(|_| HostError::new("migration-host-name-refused"))?;
    let new = CString::new(new).map_err(|_| HostError::new("migration-host-name-refused"))?;
    if unsafe {
        libc::renameat(
            directory.as_raw_fd(),
            old.as_ptr(),
            directory.as_raw_fd(),
            new.as_ptr(),
        )
    } != 0
    {
        return Err(HostError::new("migration-host-state-persist-failed"));
    }
    Ok(())
}

fn unlink_at(directory: &File, name: &str) -> Result<(), HostError> {
    let name = CString::new(name).map_err(|_| HostError::new("migration-host-name-refused"))?;
    if unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) } != 0 {
        return Err(HostError::new("migration-host-state-cleanup-failed"));
    }
    Ok(())
}

fn sync_directory(directory: &File) -> Result<(), HostError> {
    if unsafe { libc::fsync(directory.as_raw_fd()) } != 0 {
        return Err(HostError::new("migration-host-directory-fsync-failed"));
    }
    Ok(())
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}
