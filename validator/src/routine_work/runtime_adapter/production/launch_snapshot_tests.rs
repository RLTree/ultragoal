use super::*;

use super::super::launch_custody::cleanup_staged;

use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

#[test]
fn staged_program_is_bound_to_source_digest_and_explicitly_consumed() {
    let root = unique_root();
    fs::create_dir_all(&root).unwrap();
    let source_path = root.join("source");
    fs::copy("/bin/sh", &source_path).unwrap();
    fs::set_permissions(&source_path, fs::Permissions::from_mode(0o555)).unwrap();
    let source = PinnedExecutable::open_unbound(&source_path).unwrap();
    let authority = root.join(".routine-authority");
    fs::create_dir(&authority).unwrap();
    let launch = launch_root(&authority).unwrap();
    let token = ReservationToken {
        binding: AuthorityBinding {
            protocol_id: "sha256:protocol".to_owned(),
            effect_id: "sha256:effect".to_owned(),
            context_id: "sha256:context".to_owned(),
            candidate_id: "sha256:candidate".to_owned(),
            plan_id: "sha256:plan".to_owned(),
            snapshot_id: "sha256:snapshot".to_owned(),
        },
        request_id: "sha256:request".to_owned(),
        grant_id: "sha256:grant".to_owned(),
        recovery_marker: "sha256:recovery".to_owned(),
        recovery_for: None,
        reuse_only: false,
        expires_tick: u64::MAX,
    };
    let staged = stage_program(&launch, &token, &source).unwrap();
    assert_eq!(staged.executable.sha256, source.sha256);
    assert_eq!(
        staged.executable.identity_length(),
        source.identity_length()
    );
    assert_eq!(
        fs::symlink_metadata(staged.executable.path())
            .unwrap()
            .permissions()
            .mode()
            & 0o222,
        0
    );
    assert_eq!(
        fs::symlink_metadata(&staged.directory).unwrap().uid(),
        unsafe { libc::geteuid() }
    );
    cleanup_staged(&staged).unwrap();
    assert!(!staged.directory.exists());
    let staged_again = stage_program(&launch, &token, &source).unwrap();
    cleanup_staged(&staged_again).unwrap();
    assert!(!staged_again.directory.exists());
    fs::remove_dir_all(root).unwrap();
}

fn unique_root() -> PathBuf {
    let parent = std::env::var_os("CODEX_WORKTREE_TMP")
        .map(PathBuf::from)
        .expect("managed worktree tmp is required");
    parent.join(format!(
        "hul-routine-launch-snapshot-{}-{}",
        std::process::id(),
        NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
    ))
}
