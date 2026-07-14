use super::*;

pub(crate) fn mediator_error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::InvalidRequest, cause, None)
}

#[cfg(all(test, unix))]
mod tests {
    use super::super::{
        PinnedExecutable, reject_effective_user_control, validate_execution_path_immutability,
    };
    use std::ffi::CString;
    use std::fs;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_EXECUTABLE_FIXTURE: AtomicU64 = AtomicU64::new(0);

    struct OwnedExecutionPath {
        root: PathBuf,
        ancestor: PathBuf,
        executable: PathBuf,
    }

    impl OwnedExecutionPath {
        fn new(label: &str) -> Self {
            let sequence = NEXT_EXECUTABLE_FIXTURE.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "hul-routine-executable-immutability-{label}-{}-{sequence}",
                std::process::id()
            ));
            let ancestor = root.join("owned-bin");
            let executable = ancestor.join("owned-sh");
            fs::create_dir_all(&ancestor).unwrap();
            fs::write(&executable, b"#!/bin/sh\nexit 0\n").unwrap();
            fs::set_permissions(&ancestor, fs::Permissions::from_mode(0o555)).unwrap();
            fs::set_permissions(&executable, fs::Permissions::from_mode(0o555)).unwrap();
            Self {
                root,
                ancestor,
                executable,
            }
        }
    }

    impl Drop for OwnedExecutionPath {
        fn drop(&mut self) {
            let _ = fs::set_permissions(&self.ancestor, fs::Permissions::from_mode(0o755));
            let _ = fs::set_permissions(&self.executable, fs::Permissions::from_mode(0o755));
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn current_user_owned_0555_executable_is_mutable_despite_denied_write_access() {
        let fixture = OwnedExecutionPath::new("leaf");
        let effective_user_id = unsafe { libc::geteuid() };
        let metadata = fs::symlink_metadata(&fixture.executable).unwrap();
        assert_eq!(metadata.uid(), effective_user_id);
        assert_eq!(metadata.mode() & 0o777, 0o555);
        if effective_user_id != 0 {
            let encoded = CString::new(fixture.executable.as_os_str().as_bytes()).unwrap();
            assert_ne!(unsafe { libc::access(encoded.as_ptr(), libc::W_OK) }, 0);
        }
        let error = validate_execution_path_immutability(&fixture.executable).unwrap_err();
        assert_eq!(error.cause(), "mediator-executable-path-mutable");
    }

    #[test]
    fn current_user_owned_0555_ancestor_and_effective_root_are_mutable() {
        let fixture = OwnedExecutionPath::new("ancestor");
        let effective_user_id = unsafe { libc::geteuid() };
        let ancestor = fs::symlink_metadata(&fixture.ancestor).unwrap();
        assert_eq!(ancestor.uid(), effective_user_id);
        assert_eq!(ancestor.mode() & 0o777, 0o555);
        assert_eq!(
            reject_effective_user_control(&ancestor, effective_user_id)
                .unwrap_err()
                .cause(),
            "mediator-executable-path-mutable"
        );

        let system_shell = fs::symlink_metadata(Path::new("/bin/sh")).unwrap();
        assert_eq!(
            reject_effective_user_control(&system_shell, 0)
                .unwrap_err()
                .cause(),
            "mediator-executable-path-mutable"
        );
    }

    #[test]
    fn root_owned_system_shell_path_remains_eligible_for_non_root_effective_user() {
        let shell = Path::new("/bin/sh");
        let effective_user_id = unsafe { libc::geteuid() };
        let metadata = fs::symlink_metadata(shell).unwrap();
        assert_eq!(metadata.uid(), 0);
        assert_eq!(metadata.mode() & 0o022, 0);
        if effective_user_id == 0 {
            assert_eq!(
                validate_execution_path_immutability(shell)
                    .unwrap_err()
                    .cause(),
                "mediator-executable-path-mutable"
            );
            return;
        }
        assert_ne!(metadata.uid(), effective_user_id);
        validate_execution_path_immutability(shell).unwrap();
        let executable = PinnedExecutable::open_unbound(shell).unwrap();
        executable.validate().unwrap();
    }
}
