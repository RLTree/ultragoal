use super::*;

impl ReadSession {
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn revalidate(&self) -> Result<(), ContextError> {
        self.context.revalidate()?;
        self.revalidate_directories()?;
        self.observations.revalidate()?;
        self.revalidate_directories()?;
        self.context.revalidate()
    }

    pub(crate) fn charge_entry(&self) -> Result<(), ContextError> {
        let total = self.entries_visited.get().saturating_add(1);
        if total > MAX_READ_SESSION_ENTRIES {
            return Err(ContextError::PathDenied(format!(
                "read session exceeds {MAX_READ_SESSION_ENTRIES} visited entries"
            )));
        }
        self.entries_visited.set(total);
        Ok(())
    }

    pub(crate) fn target(&self, path: &Path) -> Result<PathBuf, ContextError> {
        let target = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };
        let relative = target.strip_prefix(&self.root).map_err(|_| {
            ContextError::PathDenied(format!(
                "read-session path escapes worktree: {}",
                target.display()
            ))
        })?;
        if relative.as_os_str().is_empty()
            || relative
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(ContextError::PathDenied(format!(
                "read-session path is not a normal file path: {}",
                target.display()
            )));
        }
        Ok(target)
    }

    pub(crate) fn pin_directory(&self, path: &Path) -> Result<(u64, u64), ContextError> {
        let target = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };
        let relative = target.strip_prefix(&self.root).map_err(|_| {
            ContextError::PathDenied(format!(
                "read-session directory escapes worktree: {}",
                target.display()
            ))
        })?;
        if relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(ContextError::PathDenied(format!(
                "read-session directory is not a normal path: {}",
                target.display()
            )));
        }
        if let Some(file) = self.directories.borrow().get(&target) {
            let metadata = file.metadata().map_err(|error| io_error(&target, error))?;
            return Ok((metadata.dev(), metadata.ino()));
        }
        let parent = target.parent().ok_or_else(|| {
            ContextError::PathDenied(format!("directory has no parent: {}", target.display()))
        })?;
        self.pin_directory(parent)?;
        let parent_file = self
            .directories
            .borrow()
            .get(parent)
            .ok_or_else(|| ContextError::PathDenied("pinned parent is unavailable".to_owned()))?
            .try_clone()
            .map_err(|error| io_error(parent, error))?;
        let name = target.file_name().ok_or_else(|| {
            ContextError::PathDenied(format!("directory has no name: {}", target.display()))
        })?;
        let opened = open_directory_at(&parent_file, name, &target)?;
        let metadata = opened
            .metadata()
            .map_err(|error| io_error(&target, error))?;
        let identity = (metadata.dev(), metadata.ino());
        self.directory_identities
            .borrow_mut()
            .insert(target.clone(), snapshot(&metadata));
        self.directories.borrow_mut().insert(target, opened);
        Ok(identity)
    }

    pub(crate) fn open_regular(&self, path: &Path) -> Result<File, ContextError> {
        let target = self.target(path)?;
        #[cfg(test)]
        std::thread::sleep(std::time::Duration::from_millis(
            TEST_PAUSE_BEFORE_OPEN_MS.swap(0, Ordering::SeqCst),
        ));
        let parent = target.parent().ok_or_else(|| {
            ContextError::PathDenied(format!("read target has no parent: {}", target.display()))
        })?;
        self.pin_directory(parent)?;
        let parent_file = self
            .directories
            .borrow()
            .get(parent)
            .ok_or_else(|| ContextError::PathDenied("pinned parent is unavailable".to_owned()))?
            .try_clone()
            .map_err(|error| io_error(parent, error))?;
        let name = target.file_name().ok_or_else(|| {
            ContextError::PathDenied(format!("read target has no name: {}", target.display()))
        })?;
        let file = open_regular_at(&parent_file, name, &target).or_else(|error| {
            if parent == self.root {
                open_anchored_read(&self.root, self.root_identity, &target)
            } else {
                Err(error)
            }
        })?;
        let metadata = file.metadata().map_err(|error| io_error(&target, error))?;
        if !metadata.is_file() {
            return Err(ContextError::PathDenied(format!(
                "read-session target is not a regular file: {}",
                target.display()
            )));
        }
        #[cfg(unix)]
        if metadata.nlink() != 1 {
            return Err(ContextError::PathDenied(format!(
                "read-session target has {} hard links: {}",
                metadata.nlink(),
                target.display()
            )));
        }
        Ok(file)
    }

    pub(crate) fn read_bounded(
        &self,
        path: &Path,
        maximum_bytes: u64,
    ) -> Result<Vec<u8>, ContextError> {
        self.require_read_capacity()?;
        let mut file = self.open_regular(path)?;
        let opened_before = file.metadata().map_err(|error| io_error(path, error))?;
        let path_before = fs::symlink_metadata(path).map_err(|error| io_error(path, error))?;
        #[cfg(unix)]
        if snapshot(&opened_before) != snapshot(&path_before) {
            return Err(ContextError::ConcurrentMutation(format!(
                "read-session file identity before read: {}",
                path.display()
            )));
        }
        self.observe_open_regular_identity(path, &opened_before, &file)?;
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 16 * 1024];
        loop {
            let reserved = self.reserve_read_capacity(buffer.len())?;
            let read = file
                .read(&mut buffer[..reserved])
                .map_err(|error| io_error(path, error))?;
            self.commit_read_capacity(reserved, read);
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..read]);
            if bytes.len() as u64 > maximum_bytes {
                return Err(ContextError::PathDenied(format!(
                    "read-session byte limit exceeded for {} (maximum {maximum_bytes})",
                    path.display()
                )));
            }
            #[cfg(test)]
            if bytes.len() == read {
                std::thread::sleep(std::time::Duration::from_millis(
                    TEST_PAUSE_AFTER_FIRST_READ_MS.swap(0, Ordering::SeqCst),
                ));
            }
        }
        if bytes.len() as u64 > maximum_bytes {
            return Err(ContextError::PathDenied(format!(
                "read-session byte limit exceeded for {} (maximum {maximum_bytes})",
                path.display()
            )));
        }
        let opened_after = file.metadata().map_err(|error| io_error(path, error))?;
        let path_after = fs::symlink_metadata(path).map_err(|error| io_error(path, error))?;
        #[cfg(unix)]
        if snapshot(&opened_before) != snapshot(&opened_after)
            || snapshot(&opened_after) != snapshot(&path_after)
        {
            return Err(ContextError::ConcurrentMutation(format!(
                "read-session file identity after read: {}",
                path.display()
            )));
        }
        let digest = format!("{:x}", Sha256::digest(&bytes));
        self.observe_regular(path, &digest, bytes.len() as u64, &opened_after, &file)?;
        Ok(bytes)
    }
}
