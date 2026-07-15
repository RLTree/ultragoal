use super::*;

pub(crate) fn mediator_error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::InvalidRequest, cause, None)
}

#[cfg(all(test, unix))]
mod tests {
    use super::super::PinnedExecutable;
    use std::fs;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_EXECUTABLE_FIXTURE: AtomicU64 = AtomicU64::new(0);

    fn teardown_owned_path(root: &Path, ancestor: &Path, executable: &Path) {
        fs::set_permissions(ancestor, fs::Permissions::from_mode(0o755))
            .expect("execution fixture ancestor restoration failed");
        fs::set_permissions(executable, fs::Permissions::from_mode(0o755))
            .expect("execution fixture mode restoration failed");
        fs::remove_dir_all(root).expect("execution fixture teardown failed");
        assert!(
            !root.exists(),
            "execution fixture teardown retained scope: {}",
            root.display()
        );
    }

    struct OwnedExecutionPath {
        root: PathBuf,
        ancestor: PathBuf,
        executable: PathBuf,
    }

    impl OwnedExecutionPath {
        fn new(label: &str) -> Self {
            let sequence = NEXT_EXECUTABLE_FIXTURE.fetch_add(1, Ordering::Relaxed);
            let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let root = manifest
                .parent()
                .expect("execution fixture manifest has no workspace parent")
                .join("target");
            fs::create_dir_all(&root).expect("execution fixture target directory is unavailable");
            let root = fs::canonicalize(root)
                .expect("execution fixture target directory is unavailable")
                .join(format!(
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

        fn teardown_after_assertions(&mut self) {
            teardown_owned_path(&self.root, &self.ancestor, &self.executable);
        }
    }

    #[test]
    fn current_user_owned_0555_executable_is_eligible_when_identity_is_stable() {
        let mut fixture = OwnedExecutionPath::new("leaf");
        let effective_user_id = unsafe { libc::geteuid() };
        let metadata = fs::symlink_metadata(&fixture.executable).unwrap();
        assert_eq!(metadata.uid(), effective_user_id);
        assert_eq!(metadata.mode() & 0o777, 0o555);
        let executable = PinnedExecutable::open_unbound(&fixture.executable).unwrap();
        executable.validate().unwrap();
        fixture.teardown_after_assertions();
    }

    #[test]
    fn current_user_owned_0555_ancestor_and_effective_root_are_not_authority_gates() {
        let mut fixture = OwnedExecutionPath::new("ancestor");
        let effective_user_id = unsafe { libc::geteuid() };
        let ancestor = fs::symlink_metadata(&fixture.ancestor).unwrap();
        assert_eq!(ancestor.uid(), effective_user_id);
        assert_eq!(ancestor.mode() & 0o777, 0o555);
        let system_shell = fs::symlink_metadata(Path::new("/bin/sh")).unwrap();
        assert_eq!(system_shell.uid(), 0);
        assert!(ancestor.is_dir());
        fixture.teardown_after_assertions();
    }

    #[test]
    fn root_owned_system_shell_path_remains_eligible_for_non_root_effective_user() {
        let shell = Path::new("/bin/sh");
        let metadata = fs::symlink_metadata(shell).unwrap();
        assert_eq!(metadata.uid(), 0);
        assert_eq!(metadata.mode() & 0o022, 0);
        let executable = PinnedExecutable::open_unbound(shell).unwrap();
        executable.validate().unwrap();
        executable.validate_named_path().unwrap();
    }

    #[test]
    fn fixture_drop_and_unwind_preserve_execution_scope_until_explicit_teardown() {
        let fixture = OwnedExecutionPath::new("drop-inert");
        let root = fixture.root.clone();
        let ancestor = fixture.ancestor.clone();
        let executable = fixture.executable.clone();
        drop(fixture);
        assert!(executable.exists());
        teardown_owned_path(&root, &ancestor, &executable);

        let result = std::panic::catch_unwind(|| {
            let fixture = OwnedExecutionPath::new("unwind-inert");
            let paths = (
                fixture.root.clone(),
                fixture.ancestor.clone(),
                fixture.executable.clone(),
            );
            std::panic::panic_any(paths);
        });
        let (root, ancestor, executable) = *result
            .unwrap_err()
            .downcast::<(PathBuf, PathBuf, PathBuf)>()
            .unwrap();
        assert!(executable.exists());
        teardown_owned_path(&root, &ancestor, &executable);
    }
}
