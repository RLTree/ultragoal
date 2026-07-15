use super::*;

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use super::super::launch_root::{ensure_launch_root, safe_token_name};
use super::super::launch_snapshot::launch_root;

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

#[test]
fn interrupted_partial_stage_recovers_without_digest_prerequisite() {
    let root = unique_root();
    fs::create_dir_all(&root).unwrap();
    let authority = root.join(".routine-authority");
    fs::create_dir(&authority).unwrap();
    let launch = launch_root(&authority).unwrap();
    ensure_launch_root(&launch).unwrap();
    let grant = "sha256:partial-grant";
    let recovery = "sha256:partial-recovery";
    let child = launch.join(format!("launch-{}", safe_token_name(grant)));
    fs::create_dir(&child).unwrap();
    fs::set_permissions(&child, fs::Permissions::from_mode(0o700)).unwrap();
    let program = child.join("program");
    let _file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .mode(0o500)
        .open(&program)
        .unwrap();
    let marker = child.join("authority");
    let mut marker_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .mode(0o400)
        .open(&marker)
        .unwrap();
    write!(marker_file, "{grant}\n{recovery}\n").unwrap();
    marker_file.sync_all().unwrap();
    recover_staged(&launch, grant, recovery, &[]).unwrap();
    assert!(!child.exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn marker_only_and_empty_stage_subsets_are_consumed() {
    let root = unique_root();
    fs::create_dir_all(&root).unwrap();
    let authority = root.join(".routine-authority");
    fs::create_dir(&authority).unwrap();
    let launch = launch_root(&authority).unwrap();
    ensure_launch_root(&launch).unwrap();
    let grant = "sha256:subset-grant";
    let recovery = "sha256:subset-recovery";
    let child = launch.join(format!("launch-{}", safe_token_name(grant)));
    fs::create_dir(&child).unwrap();
    fs::set_permissions(&child, fs::Permissions::from_mode(0o700)).unwrap();
    let marker = child.join("authority");
    let mut marker_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .mode(0o400)
        .open(&marker)
        .unwrap();
    write!(marker_file, "{grant}\n{recovery}\n").unwrap();
    marker_file.sync_all().unwrap();
    recover_staged(&launch, grant, recovery, &[]).unwrap();
    assert!(!child.exists());

    fs::create_dir(&child).unwrap();
    fs::set_permissions(&child, fs::Permissions::from_mode(0o700)).unwrap();
    let mut marker_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .mode(0o400)
        .open(&marker)
        .unwrap();
    write!(marker_file, "{grant}\n{recovery}\n").unwrap();
    marker_file.sync_all().unwrap();
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .mode(0o500)
        .open(child.join("program"))
        .unwrap();
    recover_staged(&launch, grant, recovery, &[]).unwrap();
    assert!(!child.exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_marker_and_extra_stage_entries_are_preserved() {
    let root = unique_root();
    fs::create_dir_all(&root).unwrap();
    let authority = root.join(".routine-authority");
    fs::create_dir(&authority).unwrap();
    let launch = launch_root(&authority).unwrap();
    ensure_launch_root(&launch).unwrap();
    let grant = "sha256:invalid-grant";
    let recovery = "sha256:invalid-recovery";
    let child = launch.join(format!("launch-{}", safe_token_name(grant)));
    fs::create_dir(&child).unwrap();
    fs::set_permissions(&child, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(child.join("authority"), b"not-the-pending-record\n").unwrap();
    assert!(recover_staged(&launch, grant, recovery, &[]).is_err());
    assert_eq!(
        fs::read(child.join("authority")).unwrap(),
        b"not-the-pending-record\n"
    );
    fs::remove_dir_all(&child).unwrap();

    fs::create_dir(&child).unwrap();
    fs::set_permissions(&child, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(
        child.join("authority"),
        format!("{grant}\n{recovery}\n").as_bytes(),
    )
    .unwrap();
    fs::write(child.join("unexpected"), b"foreign").unwrap();
    assert!(recover_staged(&launch, grant, recovery, &[]).is_err());
    assert!(child.join("unexpected").exists());
    fs::remove_dir_all(root).unwrap();
}

fn unique_root() -> PathBuf {
    let parent = std::env::var_os("CODEX_WORKTREE_TMP")
        .map(PathBuf::from)
        .expect("managed worktree tmp is required");
    parent.join(format!(
        "hul-routine-launch-recovery-{}-{}",
        std::process::id(),
        NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
    ))
}
