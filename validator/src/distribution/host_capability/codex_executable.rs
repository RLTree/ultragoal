use std::env;
use std::ffi::OsStr;
use std::path::PathBuf;

use crate::distribution::host_effect::PinnedHostExecutable;

const CODEX_PROGRAM: &str = "codex";
const MAX_PATH_BYTES: usize = 64 * 1024;

pub(crate) fn resolve_codex_executable() -> Result<PinnedHostExecutable, DistributionError> {
    let path = env::var_os("PATH").ok_or_else(|| error(DistributionErrorId::ObjectUnavailable))?;
    if path_byte_length(&path) > MAX_PATH_BYTES {
        return Err(error(DistributionErrorId::ObjectTooLarge));
    }
    resolve_from_path(env::split_paths(&path))
}

fn path_byte_length(path: &OsStr) -> usize {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;

        path.as_bytes().len()
    }
    #[cfg(not(unix))]
    {
        path.to_string_lossy().len()
    }
}

fn resolve_from_path(
    paths: impl IntoIterator<Item = PathBuf>,
) -> Result<PinnedHostExecutable, DistributionError> {
    for directory in paths {
        if !directory.is_absolute() {
            continue;
        }
        let candidate = directory.join(CODEX_PROGRAM);
        if let Ok(executable) = PinnedHostExecutable::pin(&candidate) {
            return Ok(executable);
        }
    }
    Err(error(DistributionErrorId::ObjectUnavailable))
}

#[cfg(all(test, unix))]
mod tests {
    use super::{path_byte_length, resolve_from_path};
    use std::ffi::OsString;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::fs::symlink;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn path_limit_counts_operating_system_string_bytes() {
        let path = OsString::from("é".repeat(64));
        assert_eq!(path_byte_length(&path), 128);
    }

    #[test]
    fn path_resolution_accepts_a_user_local_executable_without_a_private_allowlist() {
        let root = test_root("path-resolution");
        fs::create_dir_all(&root).expect("directory");
        let target = root.join("codex-target");
        fs::write(&target, b"codex").expect("executable bytes");
        fs::set_permissions(&target, fs::Permissions::from_mode(0o755)).expect("executable mode");
        let executable = root.join("codex");
        symlink(&target, &executable).expect("codex symlink");

        assert!(resolve_from_path([root.clone()]).is_ok());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn pinned_selection_rejects_in_place_replacement_after_path_resolution() {
        let root = test_root("path-resolution-replacement");
        fs::create_dir_all(&root).expect("directory");
        let executable = root.join("codex");
        fs::write(&executable, b"first").expect("executable bytes");
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))
            .expect("executable mode");

        let pinned = resolve_from_path([root.clone()]).expect("resolved");
        fs::write(&executable, b"replacement").expect("replacement bytes");

        assert!(pinned.revalidate_for_test().is_err());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn pinned_selection_keeps_original_target_after_symlink_replacement() {
        let root = test_root("path-resolution-symlink-replacement");
        fs::create_dir_all(&root).expect("directory");
        let first = root.join("codex-first");
        let second = root.join("codex-second");
        fs::write(&first, b"first").expect("first executable bytes");
        fs::write(&second, b"second").expect("second executable bytes");
        for path in [&first, &second] {
            fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("executable mode");
        }
        let executable = root.join("codex");
        symlink(&first, &executable).expect("codex symlink");

        let pinned = resolve_from_path([root.clone()]).expect("resolved");
        fs::remove_file(&executable).expect("remove symlink");
        symlink(&second, &executable).expect("replacement symlink");

        assert!(pinned.revalidate_for_test().is_ok());
        assert_eq!(
            pinned.canonical_path_for_test(),
            first
                .canonicalize()
                .expect("canonical target")
                .display()
                .to_string()
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn path_resolution_skips_relative_and_non_executable_entries() {
        let root = test_root("path-resolution-negative");
        fs::create_dir_all(&root).expect("directory");
        let executable = root.join("codex");
        fs::write(&executable, b"codex").expect("executable bytes");
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o644))
            .expect("non-executable mode");

        assert!(resolve_from_path([PathBuf::from("relative"), root.clone()]).is_err());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn path_resolution_rejects_a_hard_linked_executable() {
        let root = test_root("path-resolution-hard-link");
        fs::create_dir_all(&root).expect("directory");
        let target = root.join("codex-target");
        fs::write(&target, b"codex").expect("executable bytes");
        fs::set_permissions(&target, fs::Permissions::from_mode(0o755)).expect("executable mode");
        fs::hard_link(&target, root.join("codex")).expect("hard link");

        assert!(resolve_from_path([root.clone()]).is_err());
        fs::remove_dir_all(root).expect("cleanup");
    }

    fn test_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "harness-ultragoal-codex-executable-{label}-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ))
    }
}
