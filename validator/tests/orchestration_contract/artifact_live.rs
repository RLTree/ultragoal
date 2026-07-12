use crate::orchestration::*;
use crate::support::*;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(1);
const OWNED: &str = "validator/src/orchestration/node_a.rs";
const READ_ONLY: &str = "docs/ultragoal-successor-live/frozen/input.json";

struct TempRoot(PathBuf);

impl TempRoot {
    fn new(label: &str) -> Self {
        let serial = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "n10-artifact-{label}-{}-{serial}",
            std::process::id()
        ));
        if path.exists() {
            fs::remove_dir_all(&path).unwrap();
        }
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn write(&self, relative: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, bytes).unwrap();
        path
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn digest_bytes(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn live_subject(
    label: &str,
) -> (
    TempRoot,
    ArtifactWorkspace,
    LeaseSpec,
    WorkPackage,
    WorkerResultV1,
) {
    let root = TempRoot::new(label);
    let bytes = b"candidate-bound artifact\n";
    root.write(OWNED, bytes);
    let workspace = ArtifactWorkspace::new(&root.0).unwrap();
    let lease = lease();
    let package = package("node-a", &[], "node_a");
    let mut result = result_for(&lease, &package);
    result.artifacts[0] = ArtifactRecord {
        path: OWNED.to_owned(),
        sha256: digest_bytes(bytes),
        byte_length: bytes.len() as u64,
    };
    (root, workspace, lease, package, result)
}

#[test]
fn live_workspace_verification_accepts_stable_exact_artifact() {
    let (_root, workspace, lease, package, result) = live_subject("positive");
    let verified = workspace.verify(&result, &lease, &package).unwrap();
    assert_eq!(verified.result_id(), result.result_id().unwrap());
    assert_eq!(verified.artifact_count(), 1);
}

#[test]
fn live_submission_verifies_before_appending_event() {
    let (_root, workspace, lease, _package, result) = live_subject("submit");
    let (mut engine, _) = engine();
    engine.grant_lease(1, lease).unwrap();
    engine.start(2, "lease-001").unwrap();
    let before = engine.event_log().events().len();
    engine.submit(3, "lease-001", &result, &workspace).unwrap();
    assert_eq!(engine.event_log().events().len(), before + 1);
}

#[test]
fn digest_or_length_substitution_appends_no_event() {
    let (root, workspace, lease, _package, mut result) = live_subject("no-event");
    result.artifacts[0].sha256 = digest('e');
    let (mut engine, _) = engine();
    engine.grant_lease(1, lease).unwrap();
    engine.start(2, "lease-001").unwrap();
    let before = engine.event_log();
    assert_eq!(
        engine
            .submit(3, "lease-001", &result, &workspace)
            .unwrap_err(),
        OrchestrationError::InvalidWorkerResult
    );
    assert_eq!(engine.event_log(), before);
    drop(root);
}

#[cfg(unix)]
#[test]
fn symlink_file_and_symlink_ancestor_substitution_fail_closed() {
    use std::os::unix::fs::symlink;

    let (root, workspace, lease, package, result) = live_subject("symlink-file");
    let target = root.write("target.rs", b"candidate-bound artifact\n");
    fs::remove_file(root.0.join(OWNED)).unwrap();
    symlink(target, root.0.join(OWNED)).unwrap();
    assert_eq!(
        workspace.verify(&result, &lease, &package).unwrap_err(),
        OrchestrationError::InvalidWorkerResult
    );

    let (root, _workspace, lease, package, result) = live_subject("symlink-dir");
    let real = root.0.join("real-orchestration");
    fs::rename(root.0.join("validator/src/orchestration"), &real).unwrap();
    symlink(&real, root.0.join("validator/src/orchestration")).unwrap();
    let workspace = ArtifactWorkspace::new(&root.0).unwrap();
    assert_eq!(
        workspace.verify(&result, &lease, &package).unwrap_err(),
        OrchestrationError::InvalidWorkerResult
    );
}

#[cfg(unix)]
#[test]
fn hardlink_identity_alias_is_rejected() {
    let (root, workspace, lease, package, mut result) = live_subject("hardlink");
    let read_only = root.0.join(READ_ONLY);
    fs::create_dir_all(read_only.parent().unwrap()).unwrap();
    fs::hard_link(root.0.join(OWNED), &read_only).unwrap();
    result.artifacts.push(ArtifactRecord {
        path: READ_ONLY.to_owned(),
        sha256: result.artifacts[0].sha256.clone(),
        byte_length: result.artifacts[0].byte_length,
    });
    assert_eq!(
        workspace.verify(&result, &lease, &package).unwrap_err(),
        OrchestrationError::InvalidWorkerResult
    );
}

#[cfg(unix)]
#[test]
fn unix_socket_special_file_fails_before_artifact_open() {
    use std::os::unix::net::UnixListener;

    let (root, workspace, lease, package, result) = live_subject("special-file");
    fs::remove_file(root.0.join(OWNED)).unwrap();
    let short = PathBuf::from(format!("/private/tmp/n10-{}-sock", std::process::id()));
    if short.exists() {
        fs::remove_file(&short).unwrap();
    }
    let listener = UnixListener::bind(&short).unwrap();
    fs::rename(&short, root.0.join(OWNED)).unwrap();
    assert_eq!(
        workspace.verify(&result, &lease, &package).unwrap_err(),
        OrchestrationError::InvalidWorkerResult
    );
    drop(listener);
}

#[test]
fn deterministic_path_swap_and_in_place_mutation_are_detected() {
    let (root, workspace, lease, package, result) = live_subject("path-swap");
    let held = root.0.join("held.rs");
    assert_eq!(
        workspace
            .verify_with_hook(&result, &lease, &package, |path| {
                fs::rename(path, &held).unwrap();
                fs::write(path, b"substituted artifact\n").unwrap();
            })
            .unwrap_err(),
        OrchestrationError::InvalidWorkerResult
    );

    let (root, workspace, lease, package, result) = live_subject("mutation");
    assert_eq!(
        workspace
            .verify_with_hook(&result, &lease, &package, |path: &Path| {
                fs::write(path, b"mutated artifact bytes\n").unwrap();
            })
            .unwrap_err(),
        OrchestrationError::InvalidWorkerResult
    );
    drop(root);
}
