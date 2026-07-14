use std::fs;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use super::owned_compile_quarantine::CleanupOutcome::{Deleted, RefusedZeroWrite};
use super::owned_compile_scratch::{FAILURE_MARKER, OwnedCompileScratch};

#[test]
fn failed_issuer_control_removes_build_tree_and_bounds_marker() {
    let mut owned_paths = None;
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let owned = OwnedCompileScratch::claim("routine-issuer-induced-failure");
        owned_paths = Some((
            owned.path().to_path_buf(),
            owned.failure_marker().to_path_buf(),
        ));
        let build = owned.path().join("target/doc/public_surface");
        fs::create_dir_all(&build).unwrap();
        fs::write(build.join("large-artifact.bin"), vec![0x5a; 64 * 1024]).unwrap();
        panic!("induced issuer proof failure");
    }));
    assert!(failure.is_err());
    let (build_root, marker) = owned_paths.unwrap();
    assert!(!build_root.exists());
    let retained = fs::read(&marker).unwrap();
    assert_eq!(retained, FAILURE_MARKER);
    assert!(retained.len() <= 128);
    fs::remove_file(&marker).unwrap();
}

#[test]
fn renamed_compile_scratch_preserves_replacement_and_bounds_owned_tree() {
    let owned = OwnedCompileScratch::claim("routine-issuer-path-substitution");
    let original = owned.path().to_path_buf();
    let renamed = original.with_file_name(format!(
        "{}-renamed",
        original.file_name().unwrap().to_string_lossy()
    ));
    let build = original.join("target/doc/public_surface");
    fs::create_dir_all(&build).unwrap();
    fs::write(build.join("large-artifact.bin"), vec![0x5a; 64 * 1024]).unwrap();
    fs::rename(&original, &renamed).unwrap();
    fs::create_dir(&original).unwrap();
    fs::write(original.join("replacement-sentinel"), b"preserve\n").unwrap();
    drop(owned);

    assert_eq!(
        fs::read(original.join("replacement-sentinel")).unwrap(),
        b"preserve\n"
    );
    assert!(
        renamed
            .join("target/doc/public_surface/large-artifact.bin")
            .exists()
    );
    assert!(tree_bytes(&renamed) <= 128 * 1024);
    fs::remove_dir_all(original).unwrap();
    fs::remove_dir_all(renamed).unwrap();
}

#[test]
fn interrupted_compile_scratch_is_bounded_and_reclaimable() {
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
            "issuer_scratch_resilience::interrupted_compile_scratch_is_bounded_and_reclaimable",
            "--exact",
        ])
        .env(CHILD, "1")
        .env(OWNED_PATH, &interrupted)
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(77));
    assert!(tree_bytes(&interrupted) <= 128 * 1024);
    assert_eq!(owned.recover_interrupted(), Deleted);
    assert!(!interrupted.exists());
}

#[test]
fn child_reported_sibling_path_does_not_select_recovery_target() {
    let mut owned = OwnedCompileScratch::claim("routine-issuer-interrupted-owner");
    let mut sibling = OwnedCompileScratch::claim("routine-issuer-reported-sibling");
    let owned_path = owned.path().to_path_buf();
    let child_reported_path = sibling.path().to_path_buf();
    fs::write(
        child_reported_path.join("unrelated-sentinel"),
        b"preserve\n",
    )
    .unwrap();

    assert_eq!(owned.recover_interrupted(), Deleted);
    assert!(!owned_path.exists());
    assert_eq!(
        fs::read(child_reported_path.join("unrelated-sentinel")).unwrap(),
        b"preserve\n"
    );
    assert_eq!(sibling.recover_interrupted(), Deleted);
}

#[test]
fn sibling_and_inode_substitution_cannot_authorize_recovery() {
    let mut owned = OwnedCompileScratch::claim("routine-issuer-owned-claim");
    let mut sibling = OwnedCompileScratch::claim("routine-issuer-attacker-sibling");
    let original = owned.path().to_path_buf();
    let attacker = sibling.path().to_path_buf();
    let renamed = original.with_file_name(format!(
        "{}-held",
        original.file_name().unwrap().to_string_lossy()
    ));
    fs::write(original.join("owned-sentinel"), b"owned\n").unwrap();
    fs::write(attacker.join("attacker-sentinel"), b"attacker\n").unwrap();
    fs::rename(&original, &renamed).unwrap();
    fs::rename(&attacker, &original).unwrap();

    assert_eq!(owned.recover_interrupted(), RefusedZeroWrite);
    assert_eq!(sibling.recover_interrupted(), RefusedZeroWrite);
    assert_eq!(
        fs::read(renamed.join("owned-sentinel")).unwrap(),
        b"owned\n"
    );
    assert_eq!(
        fs::read(original.join("attacker-sentinel")).unwrap(),
        b"attacker\n"
    );
    fs::remove_dir_all(renamed).unwrap();
    fs::remove_dir_all(original).unwrap();
}

#[test]
fn copied_marker_and_token_cannot_authorize_another_claim() {
    let mut owned = OwnedCompileScratch::claim("routine-issuer-marker-owner");
    let mut sibling = OwnedCompileScratch::claim("routine-issuer-marker-source");
    let original = owned.path().to_path_buf();
    let sibling_path = sibling.path().to_path_buf();
    let marker = original.join("OWNERSHIP.v1");
    let foreign_marker = fs::read(sibling_path.join("OWNERSHIP.v1")).unwrap();
    fs::remove_file(&marker).unwrap();
    fs::write(&marker, foreign_marker).unwrap();

    assert_eq!(owned.recover_interrupted(), RefusedZeroWrite);
    assert!(original.exists());
    assert_eq!(sibling.recover_interrupted(), Deleted);
    assert!(!sibling_path.exists());
    fs::remove_dir_all(original).unwrap();
}

#[test]
fn altered_marker_mac_cannot_use_the_parent_held_claim() {
    let mut owned = OwnedCompileScratch::claim("routine-issuer-marker-mac");
    let original = owned.path().to_path_buf();
    let marker = original.join("OWNERSHIP.v1");
    let altered = fs::read(&marker).unwrap()[0] ^ 0xff;
    let mut file = fs::OpenOptions::new().write(true).open(&marker).unwrap();
    file.seek(SeekFrom::Start(0)).unwrap();
    file.write_all(&[altered]).unwrap();
    drop(file);

    assert_eq!(owned.recover_interrupted(), RefusedZeroWrite);
    assert!(original.exists());
    fs::remove_dir_all(original).unwrap();
}

#[test]
fn replacement_marker_identity_refuses_even_with_copied_bytes() {
    let mut owned = OwnedCompileScratch::claim("routine-issuer-marker-identity");
    let original = owned.path().to_path_buf();
    let marker = original.join("OWNERSHIP.v1");
    let bytes = fs::read(&marker).unwrap();
    fs::remove_file(&marker).unwrap();
    fs::write(&marker, bytes).unwrap();

    assert_eq!(owned.recover_interrupted(), RefusedZeroWrite);
    assert!(original.exists());
    fs::remove_dir_all(original).unwrap();
}

fn tree_bytes(root: &Path) -> u64 {
    fs::read_dir(root)
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            if path.is_dir() {
                tree_bytes(&path)
            } else {
                fs::symlink_metadata(path).unwrap().len()
            }
        })
        .sum()
}
