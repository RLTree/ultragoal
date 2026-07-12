use serde_json::json;
use std::fs;

#[test]
fn source_audit_parse_rejection_is_read_only() {
    let root = audit_parse_root("source-audit-parse-observe");
    let err = crate::parse_args_from(args(&[
        "--root",
        root.to_str().expect("root path"),
        "source",
        "audit",
        "--receipt",
        "validation_artifacts/ultragoal-audit/validator-receipt.json",
        "--mode",
        "not-a-mode",
    ]))
    .expect_err("source audit parse rejection");
    assert!(err.contains("invalid source audit --mode"), "{err}");
    assert!(!root.join("validation_artifacts").exists());
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn target_repo_audit_parse_rejection_is_read_only() {
    let root = audit_parse_root("target-audit-parse-observe");
    let err = crate::parse_args_from(args(&[
        "--root",
        root.to_str().expect("root path"),
        "target-repo",
        "audit",
        "--receipt",
        "validation_artifacts/ultragoal-audit/target-receipt.json",
    ]))
    .expect_err("target audit parse rejection");
    assert!(
        err.contains("missing required argument --surface-root"),
        "{err}"
    );
    assert!(!root.join("validation_artifacts").exists());
    fs::remove_dir_all(root).expect("cleanup");
}

fn audit_parse_root(name: &str) -> std::path::PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(name);
    fs::create_dir_all(&root).expect("root");
    fs::write(root.join("owned.txt"), "owned").expect("owned");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["owned.txt"]}),
    )
    .expect("manifest");
    root
}

fn args(raw: &[&str]) -> Vec<String> {
    raw.iter().map(|arg| (*arg).to_string()).collect()
}
