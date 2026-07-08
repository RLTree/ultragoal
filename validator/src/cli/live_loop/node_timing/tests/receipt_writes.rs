use super::super::write_node_timings;

#[test]
fn node_timing_receipt_rejects_ungoverned_output_paths() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("node-timing-ungoverned-path");
    std::fs::create_dir_all(&root).expect("root");

    let err = write_node_timings(
        &root,
        std::path::Path::new("artifacts/node-timing.json"),
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "hot",
        "verified-local",
        Vec::new(),
    )
    .expect_err("ungoverned receipt rejected");
    assert!(err.contains("governed claim artifact root"), "{err}");
    assert!(err.contains("external debug only"), "{err}");

    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn node_timing_receipt_reports_validation_cache_write_failure() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("node-timing-cache-blocked");
    std::fs::create_dir_all(root.join("validation_artifacts")).expect("validation artifacts");
    std::fs::write(root.join("validation_artifacts/observability"), b"file")
        .expect("block observability directory");

    let err = write_node_timings(
        &root,
        std::path::Path::new("validation_artifacts/node-timing.json"),
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "hot",
        "verified-local",
        Vec::new(),
    )
    .expect_err("validation cache write rejected");
    assert!(err.contains("validation_artifacts/observability"), "{err}");

    std::fs::remove_dir_all(root).expect("cleanup");
}
