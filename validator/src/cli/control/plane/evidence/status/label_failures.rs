use serde_json::json;

#[test]
fn label_failures_cover_unknown_rust_and_gc_status_edges() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let candidate = crate::self_tests::boundaries::support::sha('d');
    assert_eq!(
        super::label_failures(&root, "unknown", &json!({}), &candidate),
        vec!["unknown_evidence_label:unknown"]
    );
    let rust = json!({
        "schema":"harness-ultragoal.rust-devx-receipt.v1",
        "law_id":"rust-command-loop-authority",
        "status":"fail",
        "digests":{"candidate":candidate},
        "observations":[],
        "observation_failures":[],
        "claim_ceiling":"rust_devx_observation_bound"
    });
    assert!(
        super::label_failures(&root, "rust_fast", &rust, &candidate)
            .iter()
            .any(|failure| failure == "rust_receipt_status_not_pass")
    );
    let source_audit = json!({
        "status":"fail",
        "target_revision":{"kind":"package_digest","value":candidate}
    });
    assert!(
        super::label_failures(&root, "source_audit", &source_audit, &candidate)
            .iter()
            .any(|failure| failure == "source_audit_status_not_pass")
    );
    let coverage_wrong_target = json!({
        "target_revision":{"kind":"package_digest","value":crate::self_tests::boundaries::support::sha('e')},
        "coverage":{"percent":100.0},
        "uncovered_records":[],
        "claim_ceiling":"supports_complete_claim"
    });
    assert!(
        super::label_failures(&root, "coverage", &coverage_wrong_target, &candidate)
            .iter()
            .any(|failure| failure == "coverage_target_digest_mismatch")
    );
    assert!(
        super::label_failures(
            &root,
            "install_audit",
            &json!({"schema":"wrong"}),
            &candidate
        )
        .iter()
        .any(|failure| failure.starts_with("package_surface_audit_schema:"))
    );
    let gc = json!({
        "schema":"harness-ultragoal.workspace-gc-receipt.v1",
        "law_id":"workspace-artifact-cache-garbage-collection",
        "status":"fail",
        "digests":{"candidate":candidate},
        "plan":{"id":"plan"},
        "observations":[],
        "observation_failures":[],
        "claim_ceiling":"gc_observation_bound"
    });
    assert!(
        super::label_failures(&root, "gc_plan", &gc, &candidate)
            .iter()
            .any(|failure| failure == "gc_receipt_status_not_pass")
    );
}
