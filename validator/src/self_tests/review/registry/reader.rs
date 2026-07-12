use serde_json::json;
use std::path::Path;

fn failures(root: &Path, relative: &str, digest: &str) -> Vec<crate::review::round::ReviewFailure> {
    let mut out = Vec::new();
    crate::review::round::registry::exposure_errors(
        root,
        &json!({
            "live_registry_exposure": {"path": relative, "digest": digest}
        }),
        &mut out,
    );
    out
}

fn rendered(out: &[crate::review::round::ReviewFailure]) -> String {
    out.iter()
        .map(|failure| format!("{}:{}", failure.error, failure.detail))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn bound_registry_reader_rejects_leaf_and_ancestor_swaps_after_the_single_read() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("registry-read-swap");
    let current = root.join("validation_artifacts/current");
    std::fs::create_dir_all(&current).expect("current parent");
    let path = current.join("exposure-SECRET_CANARY.json");
    let original = br#"{"schema":"original"}"#;
    std::fs::write(&path, original).expect("original exposure");
    let digest = crate::digest::bytes(original);
    let replacement = current.join("replacement.json");
    std::fs::write(&replacement, br#"{"schema":"replacement"}"#).expect("replacement");
    let swap_path = path.clone();
    crate::review::round::registry::set_after_read_hook(move || {
        std::fs::rename(&swap_path, swap_path.with_extension("old")).expect("move leaf");
        std::fs::rename(&replacement, &swap_path).expect("replace leaf");
    });
    let out = failures(
        &root,
        "validation_artifacts/current/exposure-SECRET_CANARY.json",
        &digest,
    );
    assert_eq!(
        out.first().map(|failure| failure.error.as_str()),
        Some("review_round_live_registry_artifact_mismatch")
    );
    assert_eq!(out[0].detail, "registry-exposure-identity");
    assert!(!rendered(&out).contains("SECRET_CANARY"));
    std::fs::remove_dir_all(&root).expect("cleanup leaf swap");

    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("registry-parent-swap");
    let artifacts = root.join("validation_artifacts");
    let current = artifacts.join("current");
    let replacement = artifacts.join("replacement");
    std::fs::create_dir_all(&current).expect("current");
    std::fs::create_dir_all(&replacement).expect("replacement");
    std::fs::write(current.join("exposure.json"), original).expect("original");
    std::fs::write(
        replacement.join("exposure.json"),
        br#"{"schema":"replacement"}"#,
    )
    .expect("replacement");
    let swap_current = current.clone();
    crate::review::round::registry::set_after_read_hook(move || {
        std::fs::rename(&swap_current, swap_current.with_extension("old")).expect("move parent");
        std::fs::rename(&replacement, &swap_current).expect("replace parent");
    });
    let out = failures(&root, "validation_artifacts/current/exposure.json", &digest);
    assert_eq!(
        out.first().map(|failure| failure.error.as_str()),
        Some("review_round_live_registry_artifact_mismatch")
    );
    assert_eq!(out[0].detail, "registry-exposure-identity");
    std::fs::remove_dir_all(root).expect("cleanup parent swap");
}

#[test]
fn registry_reader_rejects_symlink_hardlink_and_special_targets_without_echo() {
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::symlink;

    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("registry-read-kinds");
    let safe = root.join("safe");
    std::fs::create_dir_all(&safe).expect("safe");
    let source = safe.join("source.json");
    std::fs::write(&source, br#"{"safe":true}"#).expect("source");
    let digest = crate::digest::file(&source).expect("source digest");

    let hardlink = root.join("hardlink-SECRET_CANARY.json");
    std::fs::hard_link(&source, &hardlink).expect("hardlink");
    let leaf_link = root.join("leaf-SECRET_CANARY.json");
    symlink(&source, &leaf_link).expect("leaf symlink");
    let ancestor = root.join("ancestor-SECRET_CANARY");
    symlink(&safe, &ancestor).expect("ancestor symlink");
    let fifo = root.join("fifo-SECRET_CANARY.json");
    let fifo_name = std::ffi::CString::new(fifo.as_os_str().as_bytes()).expect("fifo name");
    assert_eq!(unsafe { libc::mkfifo(fifo_name.as_ptr(), 0o600) }, 0);

    for relative in [
        "hardlink-SECRET_CANARY.json",
        "leaf-SECRET_CANARY.json",
        "ancestor-SECRET_CANARY/source.json",
        "fifo-SECRET_CANARY.json",
    ] {
        let out = failures(&root, relative, &digest);
        assert_eq!(
            out.first().map(|failure| failure.error.as_str()),
            Some("review_round_live_registry_artifact_invalid"),
            "{relative}: {}",
            rendered(&out)
        );
        assert!(!rendered(&out).contains("SECRET_CANARY"));
    }
    std::fs::remove_dir_all(root).expect("cleanup file kinds");
}

#[test]
fn registry_reader_rejects_duplicate_keys_and_oversize_without_consuming_authority() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("registry-read-bounds");
    std::fs::create_dir_all(root.join("validation_artifacts")).expect("artifacts");
    let duplicate = root.join("validation_artifacts/SECRET_CANARY-duplicate.json");
    let bytes = br#"{"outer":{"role":"one","role":"SECRET_CANARY"}}"#;
    std::fs::write(&duplicate, bytes).expect("duplicate JSON");
    let out = failures(
        &root,
        "validation_artifacts/SECRET_CANARY-duplicate.json",
        &crate::digest::bytes(bytes),
    );
    assert_eq!(
        out.first().map(|failure| failure.error.as_str()),
        Some("review_round_live_registry_artifact_malformed")
    );
    assert_eq!(out[0].detail, "registry-exposure-json");
    assert!(!rendered(&out).contains("SECRET_CANARY"));

    let oversized = root.join("validation_artifacts/SECRET_CANARY-oversized.json");
    std::fs::write(&oversized, vec![b' '; 4 * 1024 * 1024 + 1]).expect("oversized");
    let out = failures(
        &root,
        "validation_artifacts/SECRET_CANARY-oversized.json",
        crate::digest::ZERO,
    );
    assert_eq!(
        out.first().map(|failure| failure.error.as_str()),
        Some("review_round_live_registry_artifact_invalid")
    );
    assert!(!rendered(&out).contains("SECRET_CANARY"));
    std::fs::remove_dir_all(root).expect("cleanup bounds");
}

#[test]
fn registry_reader_rejects_entry_count_and_path_dimension_overflow() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("registry-dimensions");
    std::fs::create_dir_all(root.join("validation_artifacts")).expect("artifacts");
    let entries = serde_json::to_vec(&vec![serde_json::Value::Null; 16_385]).unwrap();
    std::fs::write(root.join("validation_artifacts/entries.json"), &entries).expect("entries");
    let out = failures(
        &root,
        "validation_artifacts/entries.json",
        &crate::digest::bytes(&entries),
    );
    assert_eq!(
        out.first().map(|failure| failure.error.as_str()),
        Some("review_round_live_registry_artifact_malformed")
    );

    for relative in [
        format!("{}/exposure.json", vec!["depth"; 33].join("/")),
        format!("{}.json", "x".repeat(513)),
        format!("{}/exposure.json", "x".repeat(129)),
    ] {
        let out = failures(&root, &relative, crate::digest::ZERO);
        assert_eq!(
            out.first().map(|failure| failure.error.as_str()),
            Some("review_round_live_registry_artifact_invalid"),
            "{relative}"
        );
    }
    std::fs::remove_dir_all(root).expect("cleanup dimensions");
}

#[test]
fn end_to_end_registry_session_rejects_manifest_mutation_before_final_revalidation() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("registry-session");
    super::copy_bound_inputs(&root);
    let exposure = super::fail_exposure();
    let relative = "validation_artifacts/exposure.json";
    super::write_json(&root.join(relative), &exposure);
    let digest = crate::digest::file(&root.join(relative)).expect("exposure digest");
    let manifest = root.join(".codex/agents/security-reviewer.toml");
    crate::review::round::registry::set_before_final_revalidate_hook(move || {
        let mut bytes = std::fs::read(&manifest).expect("manifest");
        bytes.extend_from_slice(b"\n");
        std::fs::write(&manifest, bytes).expect("mutate manifest");
    });
    let out = failures(&root, relative, &digest);
    assert_eq!(
        out.first().map(|failure| failure.error.as_str()),
        Some("review_round_live_registry_artifact_mismatch")
    );
    assert_eq!(out[0].detail, "registry-read-session-identity");
    std::fs::remove_dir_all(root).expect("cleanup session mutation");
}
