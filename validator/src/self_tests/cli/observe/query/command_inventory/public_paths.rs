use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

use super::{BLOCKER, STATIC_REL};

const REGISTRY: &str = "migration/generated-surface-authority.json";

fn digest(bytes: &[u8]) -> String {
    crate::digest::bytes(bytes)
        .strip_prefix("sha256:")
        .expect("digest prefix")
        .to_string()
}

fn root(label: &str, bytes: &[u8]) -> PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    fs::create_dir_all(root.join("docs/generated/observability")).expect("generated directory");
    fs::create_dir_all(root.join("migration")).expect("migration directory");
    fs::write(root.join(STATIC_REL), bytes).expect("static context");
    crate::self_tests::boundaries::workspace_fixtures::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources": [STATIC_REL]}),
    )
    .expect("manifest");
    crate::self_tests::boundaries::workspace_fixtures::write_json(
        &root.join(REGISTRY),
        &json!({
            "schema_version": "GeneratedSurfaceAuthority-v2",
            "contract_id": "harness-ultragoal-successor-contract-v2",
            "surfaces": [{
                "disposition": "retained_context",
                "output": STATIC_REL,
                "sha256": digest(bytes),
                "reason": "preserved predecessor context",
                "replacement_targets": ["HCT-OBSERVE"],
                "preserve": true,
                "physical_deletion_authorized": false
            }]
        }),
    )
    .expect("registry");
    root
}

fn metadata_snapshot(root: &Path) -> Vec<(PathBuf, u64, bool)> {
    let mut rows = walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .map(Result::unwrap)
        .map(|entry| {
            let metadata = fs::symlink_metadata(entry.path()).expect("metadata");
            (
                entry.path().strip_prefix(root).unwrap().to_path_buf(),
                metadata.len(),
                metadata.file_type().is_symlink(),
            )
        })
        .collect::<Vec<_>>();
    rows.sort();
    rows
}

fn observe(raw: &[&str]) -> crate::cli::observe::command::ObserveCommand {
    crate::cli::observe::parse(
        &raw.iter()
            .map(|item| (*item).to_string())
            .collect::<Vec<_>>(),
    )
    .expect("parse")
    .expect("observe command")
}

#[test]
fn public_current_state_and_explain_ignore_digest_bound_retained_bytes() {
    let secret = root("public-retained-secret", b"SECRET_CANARY");
    let malformed = root("public-retained-non-utf8", &[0xff, 0xfe]);
    for root in [&secret, &malformed] {
        let before = metadata_snapshot(root);
        let state = crate::cli::current_state::snapshot(root).expect("current state");
        assert_eq!(state["status"], "fail");
        assert_eq!(state["first_blocker"]["why_failed"], BLOCKER);
        assert!(!state.to_string().contains("SECRET_CANARY"));
        let command = observe(&["observe", "explain", "--next"]);
        assert_eq!(
            crate::cli::observe::run(root, &command).expect("explain"),
            1
        );
        assert_eq!(before, metadata_snapshot(root));
        assert!(!root.join("validation_artifacts").exists());
    }
    fs::remove_dir_all(secret).expect("cleanup secret");
    fs::remove_dir_all(malformed).expect("cleanup malformed");
}

#[test]
fn public_query_has_no_receipt_spool_or_export_write_by_default() {
    let root = root("public-retained-query-read", b"retained");
    let before = metadata_snapshot(&root);
    let command = observe(&[
        "observe",
        "logs",
        "query",
        "--timeout-ms",
        "1",
        "--limit",
        "0",
    ]);
    let _ = crate::cli::observe::run(&root, &command).expect("bounded query");
    assert_eq!(before, metadata_snapshot(&root));
    assert!(!root.join("validation_artifacts").exists());
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn explicit_query_receipt_writes_only_the_selected_output() {
    let root = root("public-retained-query-receipt", b"retained");
    let receipt = "validation_artifacts/observability/selected-query.json";
    let command = observe(&[
        "observe",
        "logs",
        "query",
        "--timeout-ms",
        "1",
        "--limit",
        "0",
        "--receipt",
        receipt,
    ]);
    let _ = crate::cli::observe::run(&root, &command).expect("explicit query output");
    assert!(root.join(receipt).is_file());
    assert!(
        !root
            .join("validation_artifacts/observability/spool")
            .exists()
    );
    let files = walkdir::WalkDir::new(root.join("validation_artifacts"))
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .count();
    assert_eq!(files, 1);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn missing_or_tampered_retained_context_fails_before_public_status() {
    let root = root("public-retained-invalid", b"retained");
    fs::write(root.join(STATIC_REL), b"SECRET_CANARY tampered").expect("tamper");
    let error = crate::cli::current_state::snapshot(&root).expect_err("tamper fails");
    assert!(error.contains("digest mismatch"), "{error}");
    assert!(!error.contains("SECRET_CANARY"));
    assert!(!root.join("validation_artifacts").exists());

    fs::remove_file(root.join(STATIC_REL)).expect("remove output");
    let error = crate::cli::current_state::snapshot(&root).expect_err("missing fails");
    assert!(error.contains("unavailable"), "{error}");
    assert!(!root.join("validation_artifacts").exists());
    fs::remove_dir_all(root).expect("cleanup");
}

#[cfg(unix)]
#[test]
fn symlink_and_fifo_retained_context_fail_without_read_or_write() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let root = root("public-retained-special", b"retained");
    let output = root.join(STATIC_REL);
    fs::remove_file(&output).expect("remove output");
    let outside = root.join("outside-secret");
    fs::write(&outside, "SECRET_CANARY").expect("outside");
    std::os::unix::fs::symlink(&outside, &output).expect("symlink");
    let error = crate::cli::current_state::snapshot(&root).expect_err("symlink fails");
    assert!(!error.contains("SECRET_CANARY"));

    fs::remove_file(&output).expect("remove symlink");
    let fifo = CString::new(output.as_os_str().as_bytes()).expect("fifo path");
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    assert!(crate::cli::current_state::snapshot(&root).is_err());
    assert!(!root.join("validation_artifacts").exists());
    fs::remove_dir_all(root).expect("cleanup");
}
