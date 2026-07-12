use serde_json::json;
use std::collections::BTreeMap;

#[test]
fn validator_receipt_rejects_reserved_ready_channel_and_reports_external_artifacts() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("receipt-errors");
    for dir in [
        "fixtures/valid",
        "templates",
        "validation_artifacts/ultragoal-audit",
    ] {
        std::fs::create_dir_all(root.join(dir)).expect("dir");
    }
    std::fs::write(
        root.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&json!({"resources":[]})).expect("manifest"),
    )
    .expect("manifest");
    std::fs::write(root.join("templates/RED_FIXTURES.json"), "[]").expect("red catalog");
    std::fs::write(root.join("fixtures/valid/minimal-goal-run.json"), "{}").expect("minimal goal");
    let red_report = root.join("validation_artifacts/ultragoal-audit/red-fixture-report.json");
    let stdout = root.join("validation_artifacts/ultragoal-audit/stdout.txt");
    let stderr = root.join("validation_artifacts/ultragoal-audit/stderr.txt");
    std::fs::write(&red_report, "{}").expect("red report");
    std::fs::write(&stdout, "out").expect("stdout");
    std::fs::write(&stderr, "err").expect("stderr");

    let without_examples = crate::audit::receipt::build(crate::audit::receipt::ReceiptInput {
        root: root.clone(),
        red_report: red_report.clone(),
        stdout: stdout.clone(),
        stderr: stderr.clone(),
        check_ids: Vec::new(),
        failures: BTreeMap::new(),
        red: BTreeMap::new(),
        target_artifacts: Vec::new(),
        start: "2026-06-26T00:00:02Z".to_string(),
        status: "fail".to_string(),
        validator_artifacts: Vec::new(),
        command_text: "ultragoal source audit".to_string(),
        mode: "strict".to_string(),
        scheduler_metrics: Vec::new(),
    })
    .expect("missing static examples are not readiness input");
    assert!(
        without_examples["generated_artifacts"]
            .as_array()
            .expect("generated artifacts")
            .iter()
            .all(|row| row["artifact_type"] != "ready_for_merge")
    );

    std::fs::create_dir_all(root.join("examples/generated")).expect("generated");
    let forged_ready = root.join("examples/generated/READY_FOR_MERGE-forged.json");
    std::fs::write(&forged_ready, "{").expect("malformed static ready example");
    let reserved_error = crate::audit::receipt::build(crate::audit::receipt::ReceiptInput {
        root: root.clone(),
        red_report: red_report.clone(),
        stdout: stdout.clone(),
        stderr: stderr.clone(),
        check_ids: Vec::new(),
        failures: BTreeMap::new(),
        red: BTreeMap::new(),
        target_artifacts: vec![json!({
            "artifact_type":"ready_for_merge",
            "path":"SECRET_READY_CANARY",
            "digest":"SECRET_READY_CANARY",
            "validator_run_id":"future-run"
        })],
        start: "2026-06-26T00:00:03Z".to_string(),
        status: "fail".to_string(),
        validator_artifacts: Vec::new(),
        command_text: "ultragoal source audit".to_string(),
        mode: "strict".to_string(),
        scheduler_metrics: Vec::new(),
    })
    .expect_err("raw ready-for-merge artifact rejected");
    assert_eq!(
        reserved_error,
        "reserved ready_for_merge artifact requires the typed HCT-CLAIMS channel"
    );
    assert!(!reserved_error.contains("SECRET_READY_CANARY"));

    let outside = root.with_file_name("outside-target-receipt.json");
    std::fs::write(&outside, "{}").expect("outside target");
    let receipt = crate::audit::receipt::build(crate::audit::receipt::ReceiptInput {
        root: root.clone(),
        red_report,
        stdout,
        stderr,
        check_ids: Vec::new(),
        failures: BTreeMap::new(),
        red: BTreeMap::new(),
        target_artifacts: vec![
            json!({"artifact_type":"target_repo_receipt","digest":crate::self_tests::boundaries::workspace_fixtures::sha('c')}),
            json!({
                "artifact_type":"target_repo_receipt",
                "path": outside.to_string_lossy(),
                "digest": crate::self_tests::boundaries::workspace_fixtures::sha('d')
            }),
        ],
        start: "2026-06-26T00:00:03Z".to_string(),
        status: "fail".to_string(),
        validator_artifacts: Vec::new(),
        command_text: "ultragoal source audit".to_string(),
        mode: "strict".to_string(),
        scheduler_metrics: Vec::new(),
    })
    .expect("receipt with external artifact");
    let artifacts = receipt["generated_artifacts"]
        .as_array()
        .expect("generated");
    assert!(
        artifacts
            .iter()
            .all(|row| row["artifact_type"] != "ready_for_merge"),
        "{artifacts:?}"
    );
    assert!(
        artifacts
            .iter()
            .any(|row| row.get("path").is_none() && row["validator_run_id"] != "")
    );
    assert!(
        artifacts.iter().any(|row| {
            row["path"]
                .as_str()
                .unwrap_or("")
                .starts_with("<external-artifact:")
        }),
        "{artifacts:?}"
    );
    std::fs::remove_file(outside).expect("cleanup outside");
    std::fs::remove_dir_all(root).expect("cleanup receipt errors");
}
