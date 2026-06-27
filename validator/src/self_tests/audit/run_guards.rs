use std::path::PathBuf;

fn options(root: PathBuf) -> crate::audit::AuditOptions {
    crate::audit::AuditOptions {
        root: root.clone(),
        receipt: root.join("validation_artifacts/ultragoal-audit/validator-receipt.json"),
        red_report: None,
        target_repo: None,
        mode: "fresh-init".to_string(),
        require_observability: false,
        require_product_cohesion: false,
        command_text: "ultragoal source audit".to_string(),
    }
}

#[test]
fn audit_run_rejects_bad_red_report_basename_and_malformed_target_receipts() {
    let root = crate::self_tests::boundaries::support::temp_root("audit-run-guards");
    let mut bad_report = options(root.clone());
    bad_report.red_report = Some(root.join("validation_artifacts/ultragoal-audit/not-red.json"));
    let err = crate::audit::run(bad_report).expect_err("bad red report basename");
    assert!(err.contains("red-fixture-report.json"), "{err}");

    let err = crate::audit::validate_target_receipt(&serde_json::json!({}))
        .expect_err("malformed target receipt");
    assert!(err.contains("missing target checks"), "{err}");
    assert!(err.contains("target repo fingerprint mismatch"), "{err}");
    let default_report = crate::audit::red_report_path(&options(root.clone()));
    assert_eq!(
        default_report.file_name().and_then(|name| name.to_str()),
        Some("red-fixture-report.json")
    );
    let write_err = crate::audit::target_receipt_write_result(Err("forced write".to_string()))
        .expect_err("write error propagated");
    assert_eq!(write_err, "forced write");
    let _ = std::fs::remove_dir_all(root);
}
