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
            if &canonical != path {
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

    #[cfg(unix)]
    pub(crate) fn validate_named_path(&self) -> Result<(), RoutineError> {
        validate_execution_path_immutability(&self.path)
    }
}

#[cfg(unix)]
pub(crate) fn validate_execution_path_immutability(path: &Path) -> Result<(), RoutineError> {
    validate_execution_path(path, true)
}

#[cfg(unix)]
fn validate_program_execution_path(path: &Path) -> Result<(), RoutineError> {
    validate_execution_path(path, false)
}

#[cfg(unix)]
fn validate_execution_path(path: &Path, strict_ancestors: bool) -> Result<(), RoutineError> {
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
        if strict_ancestors {
            let encoded = std::ffi::CString::new(component.as_os_str().as_bytes())
                .map_err(|_| mediator_error("mediator-executable-path-invalid"))?;
            let access = unsafe {
                libc::faccessat(
                    libc::AT_FDCWD,
                    encoded.as_ptr(),
                    libc::W_OK,
                    libc::AT_EACCESS,
                )
            };
            if access == 0 {
                return Err(mediator_error("mediator-executable-path-mutable"));
            }
            let error = std::io::Error::last_os_error();
            if !matches!(
                error.raw_os_error(),
                Some(libc::EACCES) | Some(libc::EPERM) | Some(libc::EROFS)
            ) {
                return Err(mediator_error(
                    "mediator-executable-path-access-check-failed",
                ));
            }
        }
        current = component.parent();
    }
    Ok(())
}
