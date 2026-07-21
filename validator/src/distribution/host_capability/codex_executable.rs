use std::env;
use std::path::PathBuf;

const CODEX_PROGRAM: &str = "codex";
const MAX_PATH_BYTES: usize = 64 * 1024;

pub(crate) fn resolve_codex_executable() -> Result<PathBuf, DistributionError> {
    let path = env::var_os("PATH").ok_or_else(|| error(DistributionErrorId::ObjectUnavailable))?;
    if path.to_string_lossy().len() > MAX_PATH_BYTES {
        return Err(error(DistributionErrorId::ObjectTooLarge));
    }
    resolve_from_path(env::split_paths(&path))
}

fn resolve_from_path(
    paths: impl IntoIterator<Item = PathBuf>,
) -> Result<PathBuf, DistributionError> {
    for directory in paths {
        if !directory.is_absolute() {
            continue;
        }
        let candidate = directory.join(CODEX_PROGRAM);
        let Ok(canonical) = candidate.canonicalize() else {
            continue;
        };
        let Ok(metadata) = std::fs::symlink_metadata(&canonical) else {
            continue;
        };
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || !has_executable_mode(&metadata)
        {
            continue;
        }
        return Ok(canonical);
    }
    Err(error(DistributionErrorId::ObjectUnavailable))
}

fn has_executable_mode(metadata: &std::fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        use std::os::unix::fs::PermissionsExt;

        metadata.permissions().mode() & 0o111 != 0 && metadata.nlink() == 1
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        true
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::resolve_from_path;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::fs::symlink;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn path_resolution_accepts_a_user_local_executable_without_a_private_allowlist() {
        let root = test_root("path-resolution");
        fs::create_dir_all(&root).expect("directory");
        let target = root.join("codex-target");
        fs::write(&target, b"codex").expect("executable bytes");
        fs::set_permissions(&target, fs::Permissions::from_mode(0o755)).expect("executable mode");
        let executable = root.join("codex");
        symlink(&target, &executable).expect("codex symlink");

        assert_eq!(
            resolve_from_path([root.clone()]).expect("resolved"),
            target.canonicalize().expect("canonical target")
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
