use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::owned_compile_scratch::{
    FAILURE_MARKER, OwnedCompileScratch, SUBSTITUTION_MARKER, SUBSTITUTION_MARKER_NAME,
};

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
    let retained = fs::read(renamed.join(SUBSTITUTION_MARKER_NAME)).unwrap();
    assert_eq!(retained, SUBSTITUTION_MARKER);
    assert!(retained.len() <= 128);
    assert!(tree_bytes(&renamed) <= 128 * 1024);
    fs::remove_dir_all(original).unwrap();
    fs::remove_dir_all(renamed).unwrap();
}

#[test]
fn interrupted_compile_scratch_is_bounded_and_reclaimable() {
    const CHILD: &str = "HUL_ROUTINE_INTERRUPTED_SCRATCH_CHILD";
    const RENDEZVOUS: &str = "HUL_ROUTINE_INTERRUPTED_SCRATCH_PATH";
    if std::env::var_os(CHILD).is_some() {
        let owned = OwnedCompileScratch::claim("routine-issuer-interrupted");
        fs::write(
            owned.path().join("bounded-source.rs"),
            b"pub struct Probe;\n",
        )
        .unwrap();
        fs::write(
            std::env::var_os(RENDEZVOUS).unwrap(),
            owned.path().as_os_str().as_encoded_bytes(),
        )
        .unwrap();
        unsafe { libc::_exit(77) };
    }

    let parent = OwnedCompileScratch::claim("routine-issuer-interruption-control");
    let rendezvous = parent.path().join("child-path");
    let status = Command::new(std::env::current_exe().unwrap())
        .args([
            "issuer_scratch_resilience::interrupted_compile_scratch_is_bounded_and_reclaimable",
            "--exact",
        ])
        .env(CHILD, "1")
        .env(RENDEZVOUS, &rendezvous)
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(77));
    let interrupted = PathBuf::from(String::from_utf8(fs::read(&rendezvous).unwrap()).unwrap());
    assert!(tree_bytes(&interrupted) <= 128 * 1024);
    OwnedCompileScratch::reclaim_interrupted(&interrupted);
    assert!(!interrupted.exists());
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
