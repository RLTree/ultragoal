use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;

mod boundaries;

#[test]
fn validator_receipt_identity_and_artifact_set_digest_cover_package_surfaces() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("receipt-identity");
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
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("receipt-build");
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
        start: "2026-06-26T00:00:00Z".to_string(),
        status: "pass".to_string(),
        validator_artifacts: vec![json!({"path":"validator/src/main.rs","digest":crate::self_tests::boundaries::workspace_fixtures::sha('b')})],
        command_text: "cargo run -- source audit".to_string(),
        mode: "strict".to_string(),
        scheduler_metrics: vec![crate::scheduler::Metrics {
            task_class: "pure_read_parallel",
            worker_count: 2,
            task_count: 3,
            queue_depth: 3,
            wall_ms: 7,
            cpu_ms: None,
            memory_bytes: None,
            io_bytes: None,
            cache_mode: "declared_local",
            resource_measurement_status: "wall_time_only_cpu_memory_io_unavailable",
            deterministic_ordering: true,
            shared_validation_artifact_writes_allowed: false,
        }],
    })
    .expect("validator receipt");
    assert_eq!(receipt["status"], "pass");
    assert_eq!(receipt["checks"]["schema-valid"]["status"], "pass");
    assert_eq!(
        receipt["blocked_claim_diagnostics"][0]["surface"],
        "final_packet_proof"
    );
    assert_eq!(receipt["blocked_claim_diagnostics"][0]["status"], "blocked");
    assert!(
        receipt["blocked_claim_diagnostics"][0]["observed_failures"]
            .as_array()
            .expect("diagnostic failures")
            .iter()
            .any(|failure| failure
                .as_str()
                .unwrap_or("")
                .starts_with("final_packet_proof_missing"))
    );
    assert_eq!(
        receipt["validator_execution"]["executable_provenance"]["invocation_mode"],
        "cargo_run"
    );
    assert_eq!(
        receipt["speed_budget"]["profile"],
        "strict_local_source_audit"
    );
    assert_eq!(
        receipt["scheduler_execution"][0]["task_class"],
        "pure_read_parallel"
    );
    assert_eq!(receipt["scheduler_execution"][0]["worker_count"], 2);
    assert_eq!(receipt["scheduler_execution"][0]["cpu_ms"], Value::Null);
    assert_eq!(
        receipt["scheduler_execution"][0]["memory_bytes"],
        Value::Null
    );
    assert_eq!(receipt["scheduler_execution"][0]["io_bytes"], Value::Null);
    assert_eq!(
        receipt["scheduler_execution"][0]["resource_measurement_status"],
        "wall_time_only_cpu_memory_io_unavailable"
    );
    assert_eq!(
        receipt["scheduler_execution"][0]["shared_validation_artifact_writes_allowed"],
        false
    );
    assert_eq!(
        receipt["scheduler_execution"][0]["claim_impact"],
        "supports_source_local_scheduler_timing_only_not_readiness"
    );
    assert!(
        receipt["generated_artifacts"]
            .as_array()
            .expect("generated artifacts")
            .iter()
            .all(|row| row["artifact_type"] != "ready_for_merge")
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
        start: "2026-06-26T00:00:01Z".to_string(),
        status: "fail".to_string(),
        validator_artifacts: Vec::new(),
        command_text: "ultragoal source audit".to_string(),
        mode: "strict".to_string(),
        scheduler_metrics: Vec::new(),
    })
    .expect("direct validator receipt");
    assert_eq!(direct_receipt["checks"]["schema-valid"]["status"], "fail");
    assert_eq!(
        direct_receipt["validator_execution"]["executable_provenance"]["invocation_mode"],
        "direct_executable"
    );
    std::fs::remove_dir_all(root).expect("cleanup receipt build");
}
