use super::*;

#[cfg(test)]
pub(crate) static TEST_PAUSE_BEFORE_OPEN_MS: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
pub(crate) fn set_test_pause_before_open(milliseconds: u64) {
    TEST_PAUSE_BEFORE_OPEN_MS.store(milliseconds, Ordering::SeqCst);
}

#[cfg(test)]
impl AuthorizedPath {
    pub(crate) fn open_read(&self) -> Result<File, ContextError> {
        if self.effect != EffectClass::Read || !self.existed {
            return Err(ContextError::EffectDenied(
                "read descriptor requires an existing read-authorized path".to_owned(),
            ));
        }
        self.revalidate()?;
        let file = open_anchored(
            &self.worktree_root,
            self.worktree_identity,
            &self.canonical_path,
            false,
            false,
        )?;
        self.validate_descriptor(&file, false)?;
        Ok(file)
    }

    pub(crate) fn open_existing_write(&self) -> Result<File, ContextError> {
        self.require_write_effect()?;
        if !self.existed {
            return Err(ContextError::PathDenied(
                "existing-write descriptor cannot create a path".to_owned(),
            ));
        }
        self.revalidate()?;
        let file = open_anchored(
            &self.worktree_root,
            self.worktree_identity,
            &self.canonical_path,
            true,
            false,
        )?;
        self.validate_descriptor(&file, true)?;
        Ok(file)
    }

    pub(crate) fn create_new_write(&self) -> Result<File, ContextError> {
        self.require_write_effect()?;
        if self.existed {
            return Err(ContextError::PathDenied(
                "create-new descriptor refuses an existing path".to_owned(),
            ));
        }
        self.revalidate()?;
        let parent = self.canonical_path.parent().ok_or_else(|| {
            ContextError::PathDenied("create target has no parent directory".to_owned())
        })?;
        let parent = parent
            .canonicalize()
            .map_err(|error| io_error(parent, error))?;
        self.check_confinement(&parent)?;
        #[cfg(test)]
        std::thread::sleep(std::time::Duration::from_millis(
            TEST_PAUSE_BEFORE_OPEN_MS.swap(0, Ordering::SeqCst),
        ));
        let file = open_anchored(
            &self.worktree_root,
            self.worktree_identity,
            &self.canonical_path,
            true,
            true,
        )?;
        self.validate_descriptor(&file, true)?;
        Ok(file)
    }

    pub(crate) fn require_write_effect(&self) -> Result<(), ContextError> {
        if matches!(
            self.effect,
            EffectClass::WorkspaceWrite | EffectClass::Destructive
        ) {
            Ok(())
        } else {
            Err(ContextError::EffectDenied(format!(
                "{:?} does not authorize a write descriptor",
                self.effect
            )))
        }
    }

    pub(crate) fn validate_descriptor(
        &self,
        file: &File,
        writing: bool,
    ) -> Result<(), ContextError> {
        let resolved = self
            .canonical_path
            .canonicalize()
            .map_err(|error| io_error(&self.canonical_path, error))?;
        self.check_confinement(&resolved)?;
        let path_metadata = fs::metadata(&resolved).map_err(|error| io_error(&resolved, error))?;
        let file_metadata = file
            .metadata()
            .map_err(|error| io_error(&resolved, error))?;
        #[cfg(unix)]
        {
            if (path_metadata.dev(), path_metadata.ino())
                != (file_metadata.dev(), file_metadata.ino())
            {
                return Err(ContextError::PathDenied(
                    "opened descriptor does not match confined path identity".to_owned(),
                ));
            }
            if writing && file_metadata.nlink() != 1 {
                return Err(ContextError::PathDenied(format!(
                    "write descriptor has {} hard links",
                    file_metadata.nlink()
                )));
            }
        }
        #[cfg(not(unix))]
        if writing {
            return Err(ContextError::PathDenied(
                "descriptor-bound writes require Unix file identity support".to_owned(),
            ));
        }
        Ok(())
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub(crate) fn open_anchored(
    worktree: &Path,
    recorded_worktree_identity: Option<(u64, u64)>,
    target: &Path,
    writing: bool,
    create_new: bool,
) -> Result<File, ContextError> {
    let relative = target.strip_prefix(worktree).map_err(|_| {
        ContextError::PathDenied("authorized target is outside its worktree root".to_owned())
    })?;
    let mut components = relative.components().collect::<Vec<_>>();
    let final_component = components.pop().ok_or_else(|| {
        ContextError::PathDenied("authorized target has no final component".to_owned())
    })?;
    if !matches!(final_component, Component::Normal(_))
        || components
            .iter()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(ContextError::PathDenied(
            "authorized descriptor path contains a non-normal component".to_owned(),
        ));
    }
    let mut directory = File::open(worktree).map_err(|error| io_error(worktree, error))?;
    let opened_root = directory
        .metadata()
        .map_err(|error| io_error(worktree, error))?;
    if recorded_worktree_identity != Some((opened_root.dev(), opened_root.ino())) {
        return Err(ContextError::PathDenied(
            "opened worktree descriptor does not match authorization-time root inode".to_owned(),
        ));
    }
    for component in components {
        directory = open_directory_at(&directory, component.as_os_str(), target)?;
    }
    let name = c_name(final_component.as_os_str(), target)?;
    let mut flags = if writing { O_WRONLY } else { O_RDONLY } | O_CLOEXEC | O_NOFOLLOW;
    if create_new {
        flags |= O_CREAT | O_EXCL;
    }
    let descriptor = unsafe { openat(directory.as_raw_fd(), name.as_ptr(), flags, 0o600_i32) };
    if descriptor < 0 {
        return Err(io_error(target, std::io::Error::last_os_error()));
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

pub(crate) fn open_anchored_read(
    worktree: &Path,
    recorded_worktree_identity: Option<(u64, u64)>,
    target: &Path,
) -> Result<File, ContextError> {
    open_anchored(worktree, recorded_worktree_identity, target, false, false)
}
