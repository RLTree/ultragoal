use super::ConfinedRoot;
use crate::distribution::DistributionErrorId;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

#[test]
fn owned_cleanup_rejects_replaced_root_without_deleting_replacement() {
    let parent = std::env::var_os("CODEX_WORKTREE_TMP")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .canonicalize()
        .unwrap();
    let name = format!(
        "hul-distribution-cleanup-swap-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    let path = parent.join(&name);
    let displaced = parent.join(format!("{name}-original"));
    fs::create_dir(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    let root = ConfinedRoot::open(&path).unwrap();
    fs::rename(&path, &displaced).unwrap();
    fs::create_dir(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(path.join("preserve.txt"), b"replacement").unwrap();

    let failure = root.remove_owned().unwrap_err();

    assert_eq!(failure.id(), DistributionErrorId::ObjectChanged);
    assert_eq!(fs::read(path.join("preserve.txt")).unwrap(), b"replacement");
    fs::remove_dir_all(path).unwrap();
    fs::remove_dir_all(displaced).unwrap();
}

#[test]
fn owned_cleanup_accepts_codex_cachebuster_directory_names() {
    let parent = std::env::var_os("CODEX_WORKTREE_TMP")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .canonicalize()
        .unwrap();
    let name = format!(
        "hul-distribution-cleanup-cachebuster-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    let path = parent.join(&name);
    let cache = path.join(
        "plugins/cache/harness-ultragoal-local/harness-ultragoal/0.0.41+codex.20260824093100/runtime",
    );
    fs::create_dir_all(&cache).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(cache.join("ultragoal"), b"runtime").unwrap();
    let root = ConfinedRoot::open(&path).unwrap();

    let cleanup = root.remove_owned();
    if cleanup.is_err() && path.exists() {
        fs::remove_dir_all(&path).unwrap();
    }

    assert!(cleanup.is_ok(), "{cleanup:?}");
    assert!(!path.exists());
}

#[test]
fn cachebuster_directory_cleanup_refuses_symlink_without_following_it() {
    let parent = std::env::var_os("CODEX_WORKTREE_TMP")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .canonicalize()
        .unwrap();
    let name = format!(
        "hul-distribution-cleanup-cachebuster-link-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    let path = parent.join(&name);
    let outside = parent.join(format!("{name}-outside"));
    let cache = path.join(
        "plugins/cache/harness-ultragoal-local/harness-ultragoal/0.0.41+codex.20260824093100/runtime",
    );
    fs::create_dir_all(&cache).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(&outside, b"outside").unwrap();
    symlink(&outside, cache.join("outside-link")).unwrap();
    let root = ConfinedRoot::open(&path).unwrap();

    let failure = root.remove_owned().unwrap_err();

    assert_eq!(failure.id(), DistributionErrorId::UnsafeObject);
    assert_eq!(fs::read(&outside).unwrap(), b"outside");
    assert!(
        fs::symlink_metadata(cache.join("outside-link"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    fs::remove_dir_all(path).unwrap();
    fs::remove_file(outside).unwrap();
}
