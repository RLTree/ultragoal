use super::*;

impl PinnedExecutable {
    #[cfg(unix)]
    pub(crate) fn from_bound_file(
        path: &Path,
        mut file: File,
        expected_sha256: &str,
        expected_length: u64,
        expected_mode: u32,
    ) -> Result<Self, RoutineError> {
        file.seek(SeekFrom::Start(0))
            .map_err(|_| mediator_error("mediator-executable-seek-failed"))?;
        let identity = ObjectIdentity::from(
            &file
                .metadata()
                .map_err(|_| mediator_error("mediator-executable-metadata-failed"))?,
        );
        if !safe_executable(identity)
            || identity.length != expected_length
            || identity.mode != expected_mode
        {
            return Err(stale_binding());
        }
        let sha256 = digest_reader(&mut file, expected_length)?;
        if sha256 != expected_sha256 {
            return Err(stale_binding());
        }
        Ok(Self {
            path: path.to_path_buf(),
            file,
            identity,
            sha256,
        })
    }

    #[cfg(unix)]
    pub(crate) fn validate_bound_at(
        &self,
        directory: &File,
        name: &str,
    ) -> Result<(), RoutineError> {
        let held = ObjectIdentity::from(
            &self
                .file
                .metadata()
                .map_err(|_| mediator_error("mediator-executable-metadata-failed"))?,
        );
        let name =
            CString::new(name).map_err(|_| mediator_error("mediator-executable-path-invalid"))?;
        // SAFETY: directory is live, name is one NUL-terminated component, and
        // a successful openat returns one newly owned descriptor.
        let descriptor = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
            )
        };
        if descriptor < 0 {
            return Err(mediator_error("mediator-executable-replaced"));
        }
        // SAFETY: the successful descriptor is newly owned.
        let mut current = unsafe { File::from_raw_fd(descriptor) };
        let current_identity = ObjectIdentity::from(
            &current
                .metadata()
                .map_err(|_| mediator_error("mediator-executable-metadata-failed"))?,
        );
        let digest = digest_reader(&mut current, self.identity.length)?;
        if held != self.identity
            || current_identity != self.identity
            || digest != self.sha256
            || !safe_executable(current_identity)
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
fn safe_executable(identity: ObjectIdentity) -> bool {
    identity.mode & u32::from(libc::S_IFMT) == u32::from(libc::S_IFREG)
        && identity.links == 1
        && identity.mode & 0o111 != 0
        && identity.mode & 0o022 == 0
}

fn stale_binding() -> RoutineError {
    RoutineError::new(
        RoutineErrorId::ContextMismatch,
        "mediator-executable-binding-stale",
        None,
    )
}
