impl AnchoredDirectory {
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
