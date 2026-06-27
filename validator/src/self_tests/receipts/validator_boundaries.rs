use serde_json::json;
use std::collections::BTreeMap;

fn receipt_root(name: &str) -> std::path::PathBuf {
    let root = crate::self_tests::boundaries::support::temp_root(name);
    for dir in [
        "examples/generated",
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
    root
}

fn input(root: &std::path::Path) -> crate::audit::receipt::ReceiptInput {
    let red_report = root.join("validation_artifacts/ultragoal-audit/red-fixture-report.json");
    let stdout = root.join("validation_artifacts/ultragoal-audit/stdout.txt");
    let stderr = root.join("validation_artifacts/ultragoal-audit/stderr.txt");
    std::fs::write(&red_report, "{}").expect("red report");
    std::fs::write(&stdout, "out").expect("stdout");
    std::fs::write(&stderr, "err").expect("stderr");
    crate::audit::receipt::ReceiptInput {
        root: root.to_path_buf(),
        red_report,
        stdout,
        stderr,
        check_ids: vec!["schema-valid".to_string()],
        failures: BTreeMap::new(),
        red: BTreeMap::new(),
        target_artifacts: Vec::new(),
        start: "2026-06-26T00:00:04Z".to_string(),
        status: "fail".to_string(),
        validator_artifacts: Vec::new(),
        command_text: "ultragoal source audit".to_string(),
    }
}

#[test]
fn validator_receipt_reports_unknown_external_path_labels() {
    let root = receipt_root("receipt-unknown-external");
    let mut input = input(&root);
    input.target_artifacts = vec![json!({
        "artifact_type": "target_repo_receipt",
        "path": "/",
        "digest": crate::self_tests::boundaries::support::sha('e')
    })];
    let receipt = crate::audit::receipt::build(input).expect("receipt");
    let artifacts = receipt["generated_artifacts"]
        .as_array()
        .expect("artifacts");
    assert!(
        artifacts
            .iter()
            .any(|row| row["path"] == "<external-artifact:unknown>"),
        "{artifacts:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup unknown external");
}

#[test]
fn validator_receipt_helper_errors_are_typed_and_testable() {
    assert!(
        crate::audit::receipt::current_exe_result(Err(std::io::Error::other("missing exe")))
            .expect_err("current exe error")
            .contains("current executable lookup failed")
    );
    assert!(
        crate::audit::receipt::generated_dir_entries(Err(std::io::Error::other("missing dir")))
            .expect_err("generated dir error")
            .contains("read generated dir")
    );
    assert!(
        crate::audit::receipt::generated_entry_path(Err(std::io::Error::other("bad entry")))
            .expect_err("generated entry error")
            .contains("read generated entry")
    );
    assert!(
        crate::audit::receipt::canonical_or_original(std::path::Path::new(
            "definitely-missing-receipt-path"
        ))
        .ends_with("definitely-missing-receipt-path")
    );
}

#[test]
#[cfg(unix)]
fn validator_receipt_rejects_tampered_generated_artifacts() {
    let hard_root = receipt_root("receipt-hardlinked-red");
    let mut hard_input = input(&hard_root);
    let red_link = hard_root.join("validation_artifacts/ultragoal-audit/red-hardlink.json");
    std::fs::hard_link(&hard_input.red_report, &red_link).expect("hard link red report");
    hard_input.red_report = red_link;
    let hard_err =
        crate::audit::receipt::build(hard_input).expect_err("hard-linked red report rejected");
    assert!(hard_err.contains("hard-linked file rejected"), "{hard_err}");
    std::fs::remove_dir_all(hard_root).expect("cleanup hard root");

    let ready_root = receipt_root("receipt-ready-symlink");
    let mut ready_input = input(&ready_root);
    let real = ready_root.join("examples/generated/real-ready.json");
    std::fs::write(&real, "{}").expect("real ready");
    std::os::unix::fs::symlink(
        &real,
        ready_root.join("examples/generated/READY_FOR_MERGE.symlink.json"),
    )
    .expect("ready symlink");
    ready_input.red_report =
        ready_root.join("validation_artifacts/ultragoal-audit/red-fixture-report.json");
    let ready_err = crate::audit::receipt::build(ready_input).expect_err("ready symlink rejected");
    assert!(ready_err.contains("not a regular file"), "{ready_err}");
    std::fs::remove_dir_all(ready_root).expect("cleanup ready root");
}
