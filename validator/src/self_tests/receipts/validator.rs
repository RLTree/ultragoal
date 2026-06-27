use serde_json::json;
use std::collections::BTreeMap;
use std::path::Path;

#[test]
fn validator_receipt_identity_and_artifact_set_digest_cover_package_surfaces() {
    let root = crate::self_tests::boundaries::support::temp_root("receipt-identity");
    let installed = root.join(".codex/plugins/harness-ultragoal");
    let cache = root.join(".codex/plugins/cache/local-harness-plugins/harness-ultragoal/0.0.11");
    let other = root.join("other-package");
    std::fs::create_dir_all(&installed).expect("installed");
    std::fs::create_dir_all(&cache).expect("cache");
    std::fs::create_dir_all(&other).expect("other");

    assert_eq!(
        crate::audit::receipt::root_identity(&installed),
        "codex-installed-plugin:harness-ultragoal"
    );
    assert_eq!(
        crate::audit::receipt::root_identity(&cache),
        "codex-plugin-cache:local-harness-plugins/harness-ultragoal/0.0.11"
    );
    assert_eq!(
        crate::audit::receipt::root_identity(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("repo root")
        ),
        "source-workspace:harness-ultragoal-plugin-proposal"
    );
    assert!(crate::audit::receipt::root_identity(&other).starts_with("package-root:sha256:"));

    let a = crate::audit::receipt::source_artifact_set_digest(&[
        json!({"path":"b","digest":"sha256:2"}),
        json!({"path":"a","digest":"sha256:1"}),
    ]);
    let b = crate::audit::receipt::source_artifact_set_digest(&[
        json!({"path":"a","digest":"sha256:1"}),
        json!({"path":"b","digest":"sha256:2"}),
    ]);
    assert_eq!(a, b);
    let c = crate::audit::receipt::source_artifact_set_digest(&[json!({})]);
    assert_ne!(a, c);
    std::fs::remove_dir_all(root).expect("cleanup receipt identity");
}

#[test]
fn validator_receipt_builds_execution_and_generated_artifacts() {
    let root = crate::self_tests::boundaries::support::temp_root("receipt-build");
    for dir in [
        "schemas",
        "templates",
        "fixtures/valid",
        "examples/generated",
        "validation_artifacts/ultragoal-audit",
    ] {
        std::fs::create_dir_all(root.join(dir)).expect("dir");
    }
    std::fs::write(
        root.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&json!({"resources":[]})).expect("manifest"),
    )
    .expect("manifest");
    std::fs::write(root.join("schemas/schema-catalog.json"), "[]").expect("schema catalog");
    std::fs::write(root.join("templates/RED_FIXTURES.json"), "[]").expect("red catalog");
    std::fs::write(root.join("fixtures/valid/minimal-goal-run.json"), "{}").expect("minimal goal");
    std::fs::write(
        root.join("examples/generated/READY_FOR_MERGE.valid.json"),
        "{}",
    )
    .expect("ready artifact");
    std::fs::write(
        root.join("examples/generated/READY_FOR_MERGE.alpha.json"),
        "{}",
    )
    .expect("second ready artifact");
    std::fs::write(root.join("examples/generated/OTHER.json"), "{}").expect("other artifact");
    let red_report = root.join("validation_artifacts/ultragoal-audit/red-fixture-report.json");
    let stdout = root.join("validation_artifacts/ultragoal-audit/stdout.txt");
    let stderr = root.join("validation_artifacts/ultragoal-audit/stderr.txt");
    std::fs::write(&red_report, "{}").expect("red report");
    std::fs::write(&stdout, "out").expect("stdout");
    std::fs::write(&stderr, "err").expect("stderr");

    let receipt = crate::audit::receipt::build(crate::audit::receipt::ReceiptInput {
        root: root.clone(),
        red_report,
        stdout,
        stderr,
        check_ids: vec!["schema-valid".to_string()],
        failures: BTreeMap::from([("schema-valid".to_string(), Vec::new())]),
        red: BTreeMap::new(),
        target_artifacts: vec![json!({
            "artifact_type":"target_repo_receipt",
            "path":root.join("target.json").to_string_lossy(),
            "digest":crate::self_tests::boundaries::support::sha('a')
        })],
        start: "2026-06-26T00:00:00Z".to_string(),
        status: "pass".to_string(),
        validator_artifacts: vec![json!({"path":"validator/src/main.rs","digest":crate::self_tests::boundaries::support::sha('b')})],
        command_text: "cargo run -- source audit".to_string(),
    })
    .expect("validator receipt");
    assert_eq!(receipt["status"], "pass");
    assert_eq!(receipt["checks"]["schema-valid"]["status"], "pass");
    assert_eq!(
        receipt["validator_execution"]["executable_provenance"]["invocation_mode"],
        "cargo_run"
    );
    assert!(
        receipt["generated_artifacts"]
            .as_array()
            .expect("generated artifacts")
            .iter()
            .any(|row| row["artifact_type"] == "ready_for_merge")
    );
    assert!(
        !receipt["generated_artifacts"]
            .as_array()
            .expect("generated artifacts")
            .iter()
            .any(|row| row["path"].as_str().unwrap_or("").ends_with("OTHER.json"))
    );
    let direct_receipt = crate::audit::receipt::build(crate::audit::receipt::ReceiptInput {
        root: root.clone(),
        red_report: root.join("validation_artifacts/ultragoal-audit/red-fixture-report.json"),
        stdout: root.join("validation_artifacts/ultragoal-audit/stdout.txt"),
        stderr: root.join("validation_artifacts/ultragoal-audit/stderr.txt"),
        check_ids: vec!["schema-valid".to_string()],
        failures: BTreeMap::from([("schema-valid".to_string(), vec!["bad".to_string()])]),
        red: BTreeMap::new(),
        target_artifacts: Vec::new(),
        start: "2026-06-26T00:00:01Z".to_string(),
        status: "fail".to_string(),
        validator_artifacts: Vec::new(),
        command_text: "ultragoal source audit".to_string(),
    })
    .expect("direct validator receipt");
    assert_eq!(direct_receipt["checks"]["schema-valid"]["status"], "fail");
    assert_eq!(
        direct_receipt["validator_execution"]["executable_provenance"]["invocation_mode"],
        "direct_executable"
    );
    std::fs::remove_dir_all(root).expect("cleanup receipt build");
}

#[test]
fn validator_receipt_reports_missing_generated_dir_and_external_artifacts() {
    let root = crate::self_tests::boundaries::support::temp_root("receipt-errors");
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

    let err = crate::audit::receipt::build(crate::audit::receipt::ReceiptInput {
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
    })
    .expect_err("missing generated dir rejected");
    assert!(err.contains("read generated dir"), "{err}");

    std::fs::create_dir_all(root.join("examples/generated")).expect("generated");
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
            json!({"artifact_type":"target_repo_receipt","digest":crate::self_tests::boundaries::support::sha('c')}),
            json!({
                "artifact_type":"target_repo_receipt",
                "path": outside.to_string_lossy(),
                "digest": crate::self_tests::boundaries::support::sha('d')
            }),
        ],
        start: "2026-06-26T00:00:03Z".to_string(),
        status: "fail".to_string(),
        validator_artifacts: Vec::new(),
        command_text: "ultragoal source audit".to_string(),
    })
    .expect("receipt with external artifact");
    let artifacts = receipt["generated_artifacts"]
        .as_array()
        .expect("generated");
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
