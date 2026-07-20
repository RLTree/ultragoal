use super::*;

impl PinnedExecutable {
    pub(crate) fn open_bound(
        path_hex: &str,
        expected_sha256: &str,
        expected_length: u64,
        expected_mode: Option<u32>,
    ) -> Result<Self, RoutineError> {
        let path = decode_path(path_hex)?;
        Self::open_bound_path(&path, expected_sha256, expected_length, expected_mode)
    }

    pub(crate) fn open_bound_path(
        path: &Path,
        expected_sha256: &str,
        expected_length: u64,
        expected_mode: Option<u32>,
    ) -> Result<Self, RoutineError> {
        let executable = Self::open_unbound(path)?;
        if executable.sha256 != expected_sha256
            || executable.identity_length() != expected_length
            || executable.identity_mode() != expected_mode
        {
            return Err(RoutineError::new(
                RoutineErrorId::ContextMismatch,
                "mediator-executable-binding-stale",
                None,
            ));
        }
        Ok(executable)
    }

    pub(crate) fn open_unbound(path: &Path) -> Result<Self, RoutineError> {
        #[cfg(not(unix))]
        {
            let _ = path;
            return Err(mediator_error("mediator-unix-confinement-required"));
        }
        #[cfg(unix)]
        {
            if !path.is_absolute() {
                return Err(mediator_error("mediator-executable-path-invalid"));
            }
            validate_program_execution_path(path)?;
            let metadata = fs::symlink_metadata(path)
                .map_err(|_| mediator_error("mediator-executable-unavailable"))?;
            if metadata.file_type().is_symlink()
                || !metadata.is_file()
                || metadata.nlink() != 1
                || metadata.permissions().mode() & 0o111 == 0
                || metadata.permissions().mode() & 0o022 != 0
            {
                return Err(mediator_error("mediator-executable-identity-unsafe"));
            }
            let canonical = path
                .canonicalize()
                .map_err(|_| mediator_error("mediator-executable-unavailable"))?;
            if canonical != path {
                return Err(mediator_error("mediator-executable-not-canonical"));
            }
            let mut file = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open(path)
                .map_err(|_| mediator_error("mediator-executable-open-failed"))?;
            let identity = ObjectIdentity::from(
                &file
                    .metadata()
                    .map_err(|_| mediator_error("mediator-executable-metadata-failed"))?,
            );
            let sha256 = digest_reader(&mut file, u64::MAX)?;
            Ok(Self {
                path: path.to_path_buf(),
                file,
                identity,
                sha256,
            })
        }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn validate_loaded_vnode(
        &self,
        device: u64,
        inode: u64,
    ) -> Result<(), RoutineError> {
        if self.identity.device != device || self.identity.inode != inode {
            return Err(RoutineError::new(
                RoutineErrorId::ConcurrentMutation,
                "mediator-loaded-executable-replaced",
                None,
            ));
        }
        Ok(())
    }

    #[cfg(unix)]
    pub(crate) fn identity_length(&self) -> u64 {
        self.identity.length
    }

    #[cfg(not(unix))]
    pub(crate) fn identity_length(&self) -> u64 {
        0
    }

    #[cfg(unix)]
    pub(crate) fn identity_mode(&self) -> Option<u32> {
        Some(self.identity.mode)
    }

    #[cfg(not(unix))]
    pub(crate) fn identity_mode(&self) -> Option<u32> {
        None
    }

    pub(crate) fn validate(&self) -> Result<(), RoutineError> {
        #[cfg(not(unix))]
        {
            return Err(mediator_error("mediator-unix-confinement-required"));
        }
        #[cfg(unix)]
        {
            let held = ObjectIdentity::from(
                &self
                    .file
                    .metadata()
                    .map_err(|_| mediator_error("mediator-executable-metadata-failed"))?,
            );
            let current_meta = fs::symlink_metadata(&self.path)
                .map_err(|_| mediator_error("mediator-executable-replaced"))?;
            if current_meta.file_type().is_symlink() || !current_meta.is_file() {
                return Err(mediator_error("mediator-executable-replaced"));
            }
            let current = ObjectIdentity::from(&current_meta);
            let mut reopened = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open(&self.path)
                .map_err(|_| mediator_error("mediator-executable-replaced"))?;
            let digest = digest_reader(&mut reopened, u64::MAX)?;
            validate_program_execution_path(&self.path)?;
            if held != self.identity
                || current != self.identity
                || digest != self.sha256
                || current.links != self.identity.links
            {
                return Err(RoutineError::new(
                    RoutineErrorId::ConcurrentMutation,
                    "mediator-executable-replaced",
                    None,
                ));
            }
            Ok(())
        }
    }
}

#[cfg(unix)]
fn validate_program_execution_path(path: &Path) -> Result<(), RoutineError> {
    let mut current = Some(path);
    while let Some(component) = current {
        let metadata = fs::symlink_metadata(component)
            .map_err(|_| mediator_error("mediator-executable-path-unavailable"))?;
        if metadata.file_type().is_symlink()
            || (component == path && !metadata.is_file())
            || (component != path && !metadata.is_dir())
            || (component == path && metadata.permissions().mode() & 0o022 != 0)
        {
            return Err(mediator_error("mediator-executable-path-mutable"));
        }
        current = component.parent();
    }
    Ok(())
}
