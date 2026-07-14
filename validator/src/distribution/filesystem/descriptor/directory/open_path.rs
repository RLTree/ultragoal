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
        let authority = StableDirectoryAuthority::capture(&metadata)?;
        let identity = authority.identity;
        Ok(Self {
            anchor: Arc::new(Anchor {
                file,
                identity,
                authority,
            }),
            identity,
            root_device: identity.device,
            anchor_relative: String::new(),
            relative: String::new(),
        })
    }

    pub(crate) fn duplicate(&self) -> Result<Self, DistributionError> {
        self.current_descriptor()?;
        Ok(Self {
            anchor: Arc::clone(&self.anchor),
            identity: self.identity,
            root_device: self.root_device,
            anchor_relative: self.anchor_relative.clone(),
            relative: self.relative.clone(),
        })
    }

    pub(crate) fn identity(&self) -> DirectoryIdentity {
        self.identity
    }

    /// Retain this directory as the mutation anchor for one confined root.
    ///
    /// The resulting descriptor binds owner, group, and the complete mode in
    /// addition to device/inode identity. Every descendant descriptor reopen
    /// therefore revalidates mutable root authority immediately before use.
    pub(crate) fn retain_confined_root(&self) -> Result<Self, DistributionError> {
        let file = self.current_descriptor()?;
        let metadata = file
            .metadata()
            .map_err(|_| error(DistributionErrorId::ObjectChanged))?;
        let authority = StableDirectoryAuthority::confined_root(&metadata)?;
        if authority.identity != self.identity || authority.identity.device != self.root_device {
            return Err(error(DistributionErrorId::ObjectChanged));
        }
        Ok(Self {
            anchor: Arc::new(Anchor {
                file,
                identity: authority.identity,
                authority,
            }),
            identity: authority.identity,
            root_device: authority.identity.device,
            anchor_relative: String::new(),
            relative: self.relative.clone(),
        })
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
        let anchor_relative = joined(&self.anchor_relative, text);
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
            anchor_relative,
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
}
