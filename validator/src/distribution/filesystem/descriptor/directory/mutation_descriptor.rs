impl Directory {
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
            || StableDirectoryAuthority::capture(&anchor_metadata)? != self.anchor.authority
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
        for path_component in self
            .anchor_relative
            .split('/')
            .filter(|value| !value.is_empty())
        {
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
