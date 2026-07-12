use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

const STATIC_REL: &str = "docs/generated/observability/command-inventory.json";
const BLOCKER: &str = "HCT-OBSERVE successor catalog unavailable/not adopted";

mod public_paths;

fn prove_with_bait(label: &str, bait: Option<&[u8]>) -> Value {
    let root = super::super::minimal_root(label);
    if let Some(bait) = bait {
        let path = root.join(STATIC_REL);
        fs::create_dir_all(path.parent().expect("catalog parent")).expect("catalog parent");
        fs::write(path, bait).expect("catalog bait");
    }
    let health = super::command(&["observe", "stack", "health", "--run-id", "run-health"]);
    let smoke = super::command(&["observe", "stack", "smoke", "--run-id", "run-smoke"]);
    super::write_default_receipt(&root, &health, "pass");
    super::write_default_receipt(&root, &smoke, "pass");
    let prove = super::command(&["observe", "prove", "--run-id", "run-prove"]);
    let value = crate::cli::observe::telemetry::prove(&root, &prove).expect("prove receipt");
    fs::remove_dir_all(root).expect("cleanup");
    value
}

fn assert_unavailable(value: &Value) {
    assert_eq!(value["status"], "fail");
    assert_eq!(value["supported_claims"], json!([]));
    assert_eq!(
        value["claim_ceiling"],
        "observability_product_closure_failed_completion_readiness_release_update_goal_blocked"
    );
    assert!(
        value["why_failed"]
            .as_str()
            .is_some_and(|failure| failure.contains(BLOCKER)),
        "{value}"
    );
    assert!(!value.to_string().contains("SECRET_CANARY"));
}

#[test]
fn retained_static_bytes_and_absence_never_create_an_observability_claim() {
    let secret = prove_with_bait("observe-static-secret", Some(b"SECRET_CANARY"));
    let malformed = prove_with_bait("observe-static-malformed", Some(&[0xff, 0xfe]));
    let missing = prove_with_bait("observe-static-missing", None);
    for value in [&secret, &malformed, &missing] {
        assert_unavailable(value);
    }
    assert_eq!(secret["why_failed"], malformed["why_failed"]);
    assert_eq!(malformed["why_failed"], missing["why_failed"]);
    assert_eq!(secret["next_repair"], missing["next_repair"]);
}

fn metadata_snapshot(root: &Path) -> Vec<(PathBuf, u64, bool)> {
    let mut rows = walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .map(Result::unwrap)
        .map(|entry| {
            let metadata = fs::symlink_metadata(entry.path()).expect("metadata");
            (
                entry
                    .path()
                    .strip_prefix(root)
                    .expect("relative")
                    .to_path_buf(),
                metadata.len(),
                metadata.file_type().is_symlink(),
            )
        })
        .collect::<Vec<_>>();
    rows.sort();
    rows
}

#[cfg(unix)]
#[test]
fn special_static_catalog_paths_are_unread_and_current_state_writes_nothing() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let root = super::super::minimal_root("observe-static-special-paths");
    let path = root.join(STATIC_REL);
    fs::create_dir_all(path.parent().expect("catalog parent")).expect("catalog parent");
    let outside = root.with_file_name("observe-static-special-secret");
    fs::write(&outside, "SECRET_CANARY").expect("outside canary");
    std::os::unix::fs::symlink(&outside, &path).expect("symlink bait");
    let before = metadata_snapshot(&root);
    let symlink = crate::cli::current_state::snapshot_for_candidate(&root, "candidate".into());
    assert_eq!(before, metadata_snapshot(&root));
    assert_eq!(symlink["status"], "fail");
    assert_eq!(symlink["first_blocker"]["why_failed"], BLOCKER);
    assert!(!symlink.to_string().contains("SECRET_CANARY"));

    fs::remove_file(&path).expect("remove symlink");
    let fifo_name = CString::new(path.as_os_str().as_bytes()).expect("fifo name");
    assert_eq!(unsafe { libc::mkfifo(fifo_name.as_ptr(), 0o600) }, 0);
    let before = metadata_snapshot(&root);
    let fifo = crate::cli::current_state::snapshot_for_candidate(&root, "candidate".into());
    assert_eq!(before, metadata_snapshot(&root));
    assert_eq!(fifo["first_blocker"]["why_failed"], BLOCKER);
    assert_eq!(symlink["first_blocker"], fifo["first_blocker"]);
    fs::remove_file(outside).expect("outside cleanup");
    fs::remove_dir_all(root).expect("cleanup");
}
