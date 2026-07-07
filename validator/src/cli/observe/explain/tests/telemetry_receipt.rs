use super::*;

#[test]
fn explain_fails_when_base_telemetry_receipt_cannot_be_written() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "observe-explain-base-receipt-write-failure",
    );
    fs::create_dir_all(&root).expect("root");
    fs::write(root.join("owned.txt"), "owned").expect("owned");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["owned.txt"]}),
    )
    .expect("manifest");
    fs::create_dir_all(root.join("validation_artifacts/observability/spool/events.jsonl"))
        .expect("spool path blocker");
    let command =
        crate::cli::observe::parse(&["observe".to_string(), "explain-failure".to_string()])
            .expect("parse")
            .expect("observe");

    let err = run(&root, &command).expect_err("spool write failure blocks explain receipt");

    assert!(
        err.contains("events.jsonl") || err.contains("json") || err.contains("directory"),
        "{err}"
    );
    fs::remove_dir_all(root).expect("cleanup explain spool failure");
}
