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
}
