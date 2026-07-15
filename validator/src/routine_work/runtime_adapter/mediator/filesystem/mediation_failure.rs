use super::*;

pub(crate) fn mediator_error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::InvalidRequest, cause, None)
}

#[cfg(all(test, unix))]
mod tests {
    use super::super::PinnedExecutable;
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
    fn current_user_owned_0555_executable_is_eligible_when_identity_is_stable() {
        let fixture = OwnedExecutionPath::new("leaf");
        let effective_user_id = unsafe { libc::geteuid() };
        let metadata = fs::symlink_metadata(&fixture.executable).unwrap();
        assert_eq!(metadata.uid(), effective_user_id);
        assert_eq!(metadata.mode() & 0o777, 0o555);
        let executable = PinnedExecutable::open_unbound(&fixture.executable).unwrap();
        executable.validate().unwrap();
    }

    #[test]
    fn current_user_owned_0555_ancestor_and_effective_root_are_not_authority_gates() {
        let fixture = OwnedExecutionPath::new("ancestor");
        let effective_user_id = unsafe { libc::geteuid() };
        let ancestor = fs::symlink_metadata(&fixture.ancestor).unwrap();
        assert_eq!(ancestor.uid(), effective_user_id);
        assert_eq!(ancestor.mode() & 0o777, 0o555);
        let system_shell = fs::symlink_metadata(Path::new("/bin/sh")).unwrap();
        assert_eq!(system_shell.uid(), 0);
        assert!(ancestor.is_dir());
    }

    #[test]
    fn root_owned_system_shell_path_remains_eligible_for_non_root_effective_user() {
        let shell = Path::new("/bin/sh");
        let effective_user_id = unsafe { libc::geteuid() };
        let metadata = fs::symlink_metadata(shell).unwrap();
        assert_eq!(metadata.uid(), 0);
        assert_eq!(metadata.mode() & 0o022, 0);
        let executable = PinnedExecutable::open_unbound(shell).unwrap();
        executable.validate().unwrap();
        executable.validate_named_path().unwrap();
    }
}
