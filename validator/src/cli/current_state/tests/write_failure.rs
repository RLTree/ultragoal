use super::*;

#[test]
pub(crate) fn current_state_run_fails_closed_when_receipt_cannot_be_written() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("current-state-write-failure");
    std::fs::create_dir_all(&root).expect("root");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    std::fs::write(root.join("validation_artifacts"), b"file").expect("block receipt directory");
    let command = CurrentStateCommand {
        json: false,
        receipt: Some("validation_artifacts/current-state.json".into()),
    };

    let err = run(&root, &command).expect_err("receipt write must fail closed");

    assert!(
        err.contains("validation_artifacts"),
        "receipt write failure should name the blocked artifact path: {err}"
    );
    std::fs::remove_dir_all(root).expect("cleanup write failure");
}
