use super::*;

#[test]
fn read_only_explain_ignores_blocked_spool_and_writes_nothing() {
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

    let before = walkdir::WalkDir::new(&root)
        .into_iter()
        .map(Result::unwrap)
        .map(|entry| {
            let metadata = fs::symlink_metadata(entry.path()).expect("metadata");
            (
                entry.path().strip_prefix(&root).unwrap().to_path_buf(),
                metadata.len(),
            )
        })
        .collect::<Vec<_>>();
    let receipt = run(&root, &command).expect("read-only explain result");
    let after = walkdir::WalkDir::new(&root)
        .into_iter()
        .map(Result::unwrap)
        .map(|entry| {
            let metadata = fs::symlink_metadata(entry.path()).expect("metadata");
            (
                entry.path().strip_prefix(&root).unwrap().to_path_buf(),
                metadata.len(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(receipt["event"]["exporter"], "none_read_only");
    assert_eq!(receipt["receipt_path"], "none-read-only");
    assert_eq!(before, after);
    fs::remove_dir_all(root).expect("cleanup explain spool failure");
}
