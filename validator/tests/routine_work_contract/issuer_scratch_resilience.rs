use std::fs;
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::process::Command;

use super::owned_compile_retention::CleanupOutcome::{
    RefusedZeroWrite, RetainedNoDestructiveAuthority,
};
use super::owned_compile_scratch::OwnedCompileScratch;

#[test]
fn failed_issuer_control_retains_tree_for_explicit_teardown() {
    let mut owned = OwnedCompileScratch::claim("routine-issuer-induced-failure");
    let build = owned.path().join("target/doc/public_surface");
    fs::create_dir_all(&build).unwrap();
    fs::write(build.join("artifact.bin"), vec![0x5a; 64 * 1024]).unwrap();
    let failure = std::panic::catch_unwind(|| panic!("induced issuer proof failure"));
    assert!(failure.is_err());
    assert!(build.join("artifact.bin").exists());
    assert_eq!(owned.recover_interrupted(), RetainedNoDestructiveAuthority);
    owned.teardown_after_assertions();
}

#[test]
fn renamed_compile_scratch_preserves_replacement_and_owned_tree() {
    let owned = OwnedCompileScratch::claim("routine-issuer-path-substitution");
    let original = owned.path().to_path_buf();
    let renamed = original.with_extension("renamed");
    fs::write(original.join("owned"), b"owned\n").unwrap();
    fs::rename(&original, &renamed).unwrap();
    fs::create_dir(&original).unwrap();
    fs::write(original.join("replacement"), b"replacement\n").unwrap();
    drop(owned);
    assert_eq!(fs::read(renamed.join("owned")).unwrap(), b"owned\n");
    assert_eq!(
        fs::read(original.join("replacement")).unwrap(),
        b"replacement\n"
    );
    fs::remove_dir_all(original).unwrap();
    fs::remove_dir_all(renamed).unwrap();
}

#[test]
fn interrupted_compile_scratch_is_retained_for_explicit_teardown() {
    const CHILD: &str = "HUL_ROUTINE_INTERRUPTED_SCRATCH_CHILD";
    const OWNED_PATH: &str = "HUL_ROUTINE_INTERRUPTED_SCRATCH_PATH";
    if std::env::var_os(CHILD).is_some() {
        let owned = PathBuf::from(std::env::var_os(OWNED_PATH).unwrap());
        fs::write(owned.join("bounded-source.rs"), b"pub struct Probe;\n").unwrap();
        unsafe { libc::_exit(77) };
    }
    let mut owned = OwnedCompileScratch::claim("routine-issuer-interrupted");
    let interrupted = owned.path().to_path_buf();
    let status = Command::new(std::env::current_exe().unwrap())
        .args([
            "issuer_scratch_resilience::interrupted_compile_scratch_is_retained_for_explicit_teardown",
            "--exact",
        ])
        .env(CHILD, "1")
        .env(OWNED_PATH, &interrupted)
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(77));
    assert_eq!(owned.recover_interrupted(), RetainedNoDestructiveAuthority);
    assert!(interrupted.join("bounded-source.rs").exists());
    owned.teardown_after_assertions();
}

#[test]
fn sibling_path_never_selects_destructive_recovery() {
    let mut owned = OwnedCompileScratch::claim("routine-issuer-interrupted-owner");
    let mut sibling = OwnedCompileScratch::claim("routine-issuer-reported-sibling");
    fs::write(sibling.path().join("unrelated"), b"preserve\n").unwrap();
    assert_eq!(owned.recover_interrupted(), RetainedNoDestructiveAuthority);
    assert_eq!(
        fs::read(sibling.path().join("unrelated")).unwrap(),
        b"preserve\n"
    );
    owned.teardown_after_assertions();
    sibling.teardown_after_assertions();
}

#[test]
fn sibling_and_inode_substitution_refuse_without_deletion() {
    let mut owned = OwnedCompileScratch::claim("routine-issuer-owned-claim");
    let mut sibling = OwnedCompileScratch::claim("routine-issuer-attacker-sibling");
    let original = owned.path().to_path_buf();
    let attacker = sibling.path().to_path_buf();
    let held = original.with_extension("held");
    fs::write(original.join("owned"), b"owned\n").unwrap();
    fs::write(attacker.join("attacker"), b"attacker\n").unwrap();
    fs::rename(&original, &held).unwrap();
    fs::rename(&attacker, &original).unwrap();
    assert_eq!(owned.recover_interrupted(), RefusedZeroWrite);
    assert_eq!(sibling.recover_interrupted(), RefusedZeroWrite);
    assert_eq!(fs::read(held.join("owned")).unwrap(), b"owned\n");
    assert_eq!(fs::read(original.join("attacker")).unwrap(), b"attacker\n");
    fs::remove_dir_all(held).unwrap();
    fs::remove_dir_all(original).unwrap();
}

#[test]
fn copied_marker_cannot_authorize_destructive_recovery() {
    let mut owned = OwnedCompileScratch::claim("routine-issuer-marker-owner");
    let mut sibling = OwnedCompileScratch::claim("routine-issuer-marker-source");
    let marker = owned.path().join("OWNERSHIP.v1");
    let bytes = fs::read(sibling.path().join("OWNERSHIP.v1")).unwrap();
    fs::remove_file(&marker).unwrap();
    fs::write(&marker, bytes).unwrap();
    assert_eq!(owned.recover_interrupted(), RefusedZeroWrite);
    assert!(owned.path().exists());
    sibling.teardown_after_assertions();
    fs::remove_dir_all(owned.path()).unwrap();
}

#[test]
fn altered_marker_mac_refuses_without_deletion() {
    let mut owned = OwnedCompileScratch::claim("routine-issuer-marker-mac");
    let marker = owned.path().join("OWNERSHIP.v1");
    let altered = fs::read(&marker).unwrap()[0] ^ 0xff;
    let mut file = fs::OpenOptions::new().write(true).open(&marker).unwrap();
    file.seek(SeekFrom::Start(0)).unwrap();
    file.write_all(&[altered]).unwrap();
    drop(file);
    assert_eq!(owned.recover_interrupted(), RefusedZeroWrite);
    assert!(owned.path().exists());
    fs::remove_dir_all(owned.path()).unwrap();
}

#[test]
fn replacement_marker_identity_refuses_without_deletion() {
    let mut owned = OwnedCompileScratch::claim("routine-issuer-marker-identity");
    let marker = owned.path().join("OWNERSHIP.v1");
    let bytes = fs::read(&marker).unwrap();
    fs::remove_file(&marker).unwrap();
    fs::write(&marker, bytes).unwrap();
    assert_eq!(owned.recover_interrupted(), RefusedZeroWrite);
    assert!(owned.path().exists());
    fs::remove_dir_all(owned.path()).unwrap();
}
