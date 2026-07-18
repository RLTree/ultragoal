use crate::cli::observe;
use serde_json::Value;
use std::fs;

#[test]
fn observe_snapshot_refuses_semantically_empty_explain_roundtrip() {
    let root = super::super::minimal_root("observe-snapshot-empty-explain");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let target = super::write_source_audit_receipt(&root, &candidate);
    super::write_query_receipts(&root, &candidate, &target);
    super::write_explain_receipt(&root, &candidate, &target, false);
    let explain_path =
        root.join("validation_artifacts/observability/source-audit-explain-failure.json");
    let mut explain = crate::json_boundary::read_json(&explain_path).expect("explain");
    remove_field(&mut explain, "/observed_why_failed");
    remove_field(&mut explain, "/explanation/implicated_paths");
    crate::self_tests::boundaries::workspace_fixtures::write_json(&explain_path, &explain)
        .expect("write explain");

    let proof_rel = "validation_artifacts/observability/source-audit-command-roundtrip.json";
    let command = super::command(&[
        "observe",
        "snapshot",
        "--run-id",
        &target.run_id,
        "--receipt",
        proof_rel,
    ]);
    assert_eq!(observe::run(&root, &command).expect("snapshot"), 1);
    let proof = crate::json_boundary::read_json(&root.join(proof_rel)).expect("proof");
    let why = proof["why_failed"].as_str().unwrap();
    assert!(
        why.contains("explain_failure_missing_why_failed"),
        "{proof}"
    );
    assert!(
        proof["explain_evidence"]["implicated_paths"]
            .as_array()
            .is_some_and(Vec::is_empty),
        "{proof}"
    );
    fs::remove_dir_all(root).expect("cleanup snapshot empty explain");
}

fn remove_field(value: &mut Value, pointer: &str) {
    let (parent, field) = pointer.rsplit_once('/').expect("pointer");
    value
        .pointer_mut(parent)
        .and_then(Value::as_object_mut)
        .expect("object")
        .remove(field);
}
