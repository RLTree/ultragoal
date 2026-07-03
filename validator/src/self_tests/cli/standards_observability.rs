use serde_json::json;
use std::path::{Path, PathBuf};

fn write_json(path: &Path, value: &serde_json::Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec_pretty(value).expect("json")).expect("write json");
}

#[test]
fn standards_gardener_run_writes_fail_observability_without_receipt_ledger_claim() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("standards-gardener-fail");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    let command = crate::cli::standards::StandardsCommand {
        receipt: PathBuf::from("validation_artifacts/standards-gardener/missing.json"),
        observability_receipt: PathBuf::from(
            "validation_artifacts/observability/standards-gardener-rebind.json",
        ),
    };
    let code = crate::cli::standards::run(&root, &command).expect("fail command receipt");
    assert_eq!(code, 1);
    let observation = crate::json_boundary::read_json(
        &root.join("validation_artifacts/observability/standards-gardener-rebind.json"),
    )
    .expect("observability receipt");
    assert_eq!(observation["status"], "fail");
    assert_eq!(
        observation["event"]["failure_class"],
        "standards_gardener_rebind_failure"
    );
    assert!(
        observation["supported_claims"]
            .as_array()
            .expect("supported claims")
            .is_empty()
    );
    std::fs::remove_dir_all(root).expect("cleanup standards fail observe");
}
