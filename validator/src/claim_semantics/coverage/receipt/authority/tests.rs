use super::*;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_root(label: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "ultragoal-coverage-authority-{label}-{}-{stamp}",
        std::process::id()
    ))
}

#[test]
fn cached_result_reuses_success_and_error_slots() {
    let success_calls = AtomicUsize::new(0);
    let mut success = None;
    assert_eq!(
        cached_result(&mut success, || {
            success_calls.fetch_add(1, Ordering::SeqCst);
            Ok("digest".to_string())
        }),
        Ok("digest".to_string())
    );
    assert_eq!(success_calls.load(Ordering::SeqCst), 1);

    let error_calls = AtomicUsize::new(0);
    let mut error = None;
    assert_eq!(
        cached_result(&mut error, || {
            error_calls.fetch_add(1, Ordering::SeqCst);
            Err("missing".to_string())
        }),
        Err("missing".to_string())
    );
    assert_eq!(error_calls.load(Ordering::SeqCst), 1);
}

#[test]
fn check_with_cache_reuses_warm_digest_slots() {
    let root = temp_root("warm-cache");
    std::fs::create_dir_all(root.join(".harness")).expect("harness dir");
    std::fs::create_dir_all(root.join("src")).expect("src dir");
    std::fs::write(root.join("src/lib.rs"), "fn covered() {}\n").expect("src");
    std::fs::write(root.join(".harness/coverage-command"), "coverage\n").expect("command");
    std::fs::write(
        root.join(".harness/coverage-manifest.json"),
        serde_json::to_vec(&json!({
            "required_target_paths":["src"],
            "changed_file_coupling_policy":{"changed_files":["src/lib.rs"]},
            "source_discovery_rules":{"ignore":[]}
        }))
        .expect("manifest json"),
    )
    .expect("manifest");
    std::fs::write(root.join("report.json"), "{}\n").expect("report");
    let manifest_value =
        crate::json_boundary::read_json(&root.join(".harness/coverage-manifest.json"))
            .expect("manifest value");
    let receipt = json!({
        "claim_id":"COV",
        "tool_version":"llvm-cov",
        "workspace_root":"/repo",
        "command_exit":0,
        "generated_by":"coverage-command",
        "percent_source":"machine_readable_report",
        "source_tree_digest":crate::claim_semantics::coverage::digests::source_tree_digest(&root, &manifest_value).expect("source digest"),
        "coverage_manifest_digest":crate::digest::file(&root.join(".harness/coverage-manifest.json")).expect("manifest digest"),
        "coverage_command_digest":crate::digest::file(&root.join(".harness/coverage-command")).expect("command digest"),
        "changed_files_digest":crate::claim_semantics::coverage::digests::changed_files_digest(&root, &manifest_value).expect("changed digest"),
        "machine_readable_report":{"path":"report.json","digest":crate::digest::file(&root.join("report.json")).expect("report digest")}
    });
    let mut cache = DigestCache::default();
    let mut out = Vec::new();
    check_with_cache(&receipt, &root, &mut out, &mut cache);
    check_with_cache(&receipt, &root, &mut out, &mut cache);
    assert!(out.is_empty(), "{out:?}");
    std::fs::remove_dir_all(root).expect("cleanup warm cache");
}

#[test]
fn manifest_cache_reuses_loaded_and_missing_surfaces() {
    let root = temp_root("manifest-present");
    std::fs::create_dir_all(root.join(".harness")).expect("harness dir");
    std::fs::write(
        root.join(".harness/coverage-manifest.json"),
        serde_json::to_vec(&json!({"required_target_paths":[]})).expect("json"),
    )
    .expect("manifest");
    let mut cache = DigestCache::default();
    assert!(manifest(&root, &mut cache).is_some());
    assert!(manifest(&root, &mut cache).is_some());
    std::fs::remove_dir_all(&root).expect("cleanup present manifest");

    let missing = temp_root("manifest-missing");
    std::fs::create_dir_all(&missing).expect("missing root");
    let mut missing_cache = DigestCache::default();
    assert!(manifest(&missing, &mut missing_cache).is_none());
    assert!(manifest(&missing, &mut missing_cache).is_none());
    std::fs::remove_dir_all(&missing).expect("cleanup missing manifest");
}
