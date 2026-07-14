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
