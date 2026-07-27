use super::ConfinedRoot;
use crate::distribution::DistributionErrorId;
use std::fs;
use std::os::unix::fs::PermissionsExt;
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
