use serde_json::{Value, json};
use std::path::{Path, PathBuf};

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec_pretty(value).expect("json")).expect("write json");
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("copy dir");
    for entry in std::fs::read_dir(from).expect("read dir") {
        let entry = entry.expect("entry");
        let dest = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &dest);
        } else {
            std::fs::copy(entry.path(), dest).expect("copy file");
        }
    }
}

fn temp_root(label: &str) -> PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    let repo = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    copy_dir(&repo.join("schemas"), &root.join("schemas"));
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["docs/tracked.json"]}),
    );
    write_json(
        &root.join("docs/tracked.json"),
        &json!({"generated_at":"2026-06-25T00:00:00Z","value":"current"}),
    );
    root
}

fn stale_receipt() -> Value {
    json!({
        "schema": "harness-ultragoal.standards-gardening-receipt.v1",
        "status": "pass",
        "generated_at": "2026-06-25T00:00:00Z",
        "trigger_signal": {
            "signal_id": "test-signal",
            "severity": "severe",
            "signal_kind": "stale_proof_or_state",
            "summary": "test",
            "source": "unit test"
        },
        "decision": {"accepted": true, "action": "validator_check", "rationale": "test"},
        "changed_artifacts": [{
            "path":"docs/tracked.json",
            "digest": crate::self_tests::boundaries::workspace_fixtures::sha('0')
        }],
        "safeguards": {"deterministic_first": true, "no_hook_by_default": true},
        "claim_ceiling": "package_static_fixture_only"
    })
}

#[test]
fn standards_gardener_parse_rejects_unknown_and_missing_receipt() {
    let other = vec!["product".to_string()];
    assert!(
        crate::cli::standards::parse(&other)
            .expect("parse other")
            .is_none()
    );

    let unknown = vec!["standards-gardener".to_string(), "unknown".to_string()];
    let err = crate::cli::standards::parse(&unknown).expect_err("unknown command");
    assert!(err.contains("unknown standards-gardener command"));

    let missing = vec!["standards-gardener".to_string(), "rebind".to_string()];
    let err = crate::cli::standards::parse(&missing).expect_err("missing receipt");
    assert!(err.contains("missing required argument --receipt"));
}

#[test]
fn standards_gardener_rebind_mints_candidate_and_current_artifact_digest() {
    let root = temp_root("standards-gardener-rebind");
    let rel = PathBuf::from("validation_artifacts/standards-gardener/current.json");
    write_json(&root.join(&rel), &stale_receipt());
    let value = crate::cli::standards::gardener::rebind(&root, &rel).expect("rebind");
    let candidate = crate::package::inventory::package_digest(&root).expect("digest");
    let artifact_digest = crate::digest::file(&root.join("docs/tracked.json")).expect("digest");
    assert_eq!(value["candidate_digest"], candidate);
    assert_eq!(value["changed_artifacts"][0]["digest"], artifact_digest);
    let store = crate::schema_catalog::load(&root);
    assert!(crate::audit::standards_gardening::receipt_failures(&store, &value).is_empty());
    assert!(crate::audit::standards_gardening::receipt_root_failures(&root, &value).is_empty());
    std::fs::remove_dir_all(root).expect("cleanup standards rebind");
}

#[test]
fn standards_gardener_run_mints_receipt_through_cli_command() {
    let root = temp_root("standards-gardener-run");
    let rel = PathBuf::from("validation_artifacts/standards-gardener/current.json");
    write_json(&root.join(&rel), &stale_receipt());
    let command = crate::cli::standards::StandardsCommand {
        receipt: rel.clone(),
        observability_receipt: PathBuf::from("standards-gardener-rebind-observe.json"),
    };
    let code = crate::cli::standards::run(&root, &command).expect("run standards command");
    assert_eq!(code, 0);

    let value = crate::json_boundary::read_json(&root.join(rel)).expect("read rebound");
    assert_eq!(value["status"], "pass");
    assert_eq!(
        value["candidate_digest"],
        crate::package::inventory::package_digest(&root).expect("digest")
    );
    let observation =
        crate::json_boundary::read_json(&root.join("standards-gardener-rebind-observe.json"))
            .expect("observability receipt");
    assert_eq!(observation["status"], "pass");
    assert_eq!(observation["operation"], "standards-gardener.rebind");
    assert_eq!(
        observation["event"]["saturation_status"],
        "shared_authority_write_serial"
    );
    assert_eq!(
        observation["event"]["artifact_path"],
        "validation_artifacts/standards-gardener/current.json"
    );
    let store = crate::schema_catalog::load(&root);
    let errors = crate::schema_catalog::schema_errors(
        &store,
        "observability-receipt.schema.json",
        &observation,
    );
    assert!(errors.is_empty(), "{errors:?}");
    std::fs::remove_dir_all(root).expect("cleanup standards run");
}

#[test]
fn standards_gardener_rebind_rejects_unsafe_receipt_paths() {
    let root = temp_root("standards-gardener-unsafe");
    for (receipt, expected) in [
        (
            root.join("validation_artifacts/standards-gardener/current.json"),
            "root-relative under validation_artifacts/standards-gardener",
        ),
        (
            PathBuf::from("validation_artifacts/standards-gardener/../escape.json"),
            "escapes package root",
        ),
        (
            PathBuf::from("validation_artifacts/cli/not-standards.json"),
            "root-relative under validation_artifacts/standards-gardener",
        ),
    ] {
        let err = crate::cli::standards::gardener::rebind(&root, &receipt)
            .expect_err("unsafe path rejected");
        assert!(err.contains(expected), "{err}");
    }
    std::fs::remove_dir_all(root).expect("cleanup standards unsafe");
}

#[test]
fn standards_gardener_rebind_rejects_empty_and_self_tracking_artifacts() {
    let root = temp_root("standards-gardener-artifacts");
    let rel = PathBuf::from(
        "validation_artifacts/standards-gardener/current-standards-gardening-receipt.json",
    );

    let mut missing_rows = stale_receipt();
    missing_rows
        .as_object_mut()
        .expect("object")
        .remove("changed_artifacts");
    write_json(&root.join(&rel), &missing_rows);
    let err = crate::cli::standards::gardener::rebind(&root, &rel)
        .expect_err("missing changed artifacts rejected");
    assert!(err.contains("missing changed_artifacts"), "{err}");

    let mut empty = stale_receipt();
    empty["changed_artifacts"] = json!([]);
    write_json(&root.join(&rel), &empty);
    let err =
        crate::cli::standards::gardener::rebind(&root, &rel).expect_err("empty artifacts rejected");
    assert!(err.contains("changed_artifacts is empty"), "{err}");

    let mut self_tracking = stale_receipt();
    self_tracking["changed_artifacts"][0]["path"] =
        json!("validation_artifacts/standards-gardener/current-standards-gardening-receipt.json");
    write_json(&root.join(&rel), &self_tracking);
    let err =
        crate::cli::standards::gardener::rebind(&root, &rel).expect_err("self tracking rejected");
    assert!(err.contains("cannot track itself"), "{err}");

    let mut escaping = stale_receipt();
    escaping["changed_artifacts"][0]["path"] = json!("../escape.json");
    write_json(&root.join(&rel), &escaping);
    let err = crate::cli::standards::gardener::rebind(&root, &rel)
        .expect_err("escaping artifact rejected");
    assert!(err.contains("changed artifact path invalid"), "{err}");

    let mut runtime_receipt = stale_receipt();
    runtime_receipt["changed_artifacts"][0]["path"] =
        json!("validation_artifacts/cli/update-goal-eligibility.json");
    write_json(&root.join(&rel), &runtime_receipt);
    let err = crate::cli::standards::gardener::rebind(&root, &rel)
        .expect_err("runtime receipt artifact rejected");
    assert!(
        err.contains("cannot track runtime validation_artifacts"),
        "{err}"
    );

    let mut missing_path = stale_receipt();
    missing_path["changed_artifacts"][0]
        .as_object_mut()
        .expect("artifact object")
        .remove("path");
    write_json(&root.join(&rel), &missing_path);
    let err = crate::cli::standards::gardener::rebind(&root, &rel)
        .expect_err("missing artifact path rejected");
    assert!(err.contains("changed_artifact missing path"), "{err}");

    std::fs::remove_dir_all(root).expect("cleanup standards artifacts");
}

#[test]
fn standards_gardener_rebind_rejects_schema_invalid_rebound_receipt() {
    let root = temp_root("standards-gardener-invalid");
    let rel = PathBuf::from("validation_artifacts/standards-gardener/current.json");
    let mut invalid = stale_receipt();
    invalid.as_object_mut().expect("object").remove("decision");
    write_json(&root.join(&rel), &invalid);
    let err = crate::cli::standards::gardener::rebind(&root, &rel)
        .expect_err("schema invalid receipt rejected");
    assert!(err.contains("receipt did not validate"), "{err}");
    std::fs::remove_dir_all(root).expect("cleanup standards invalid");
}

#[test]
fn standards_gardener_copy_dir_preserves_nested_inputs() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("standards-copy-nested");
    let from = root.join("from");
    let to = root.join("to");
    std::fs::create_dir_all(from.join("nested")).expect("nested input");
    std::fs::write(from.join("nested/file.json"), "{}").expect("nested file");
    copy_dir(&from, &to);
    assert!(to.join("nested/file.json").is_file());
    std::fs::remove_dir_all(root).expect("cleanup standards nested copy");
}
