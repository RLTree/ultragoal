use std::fs;
use std::path::Path;

#[test]
fn claim_artifact_path_rejects_external_claim_outputs() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("claim-output-paths");
    fs::create_dir_all(&root).expect("root");

    let relative = crate::output_path::claim_artifact_path(
        &root,
        Path::new("validation_artifacts/observability/receipt.json"),
        "receipt",
    )
    .expect("root-relative claim artifact");
    assert_eq!(
        relative,
        root.join("validation_artifacts/observability/receipt.json")
    );

    let absolute = crate::output_path::claim_artifact_path(
        &root,
        &root.join("outside/receipt.json"),
        "receipt",
    )
    .expect_err("absolute output cannot support claims");
    assert!(absolute.contains("root-relative claim artifact path"));
    assert!(absolute.contains("external debug only"));

    let traversal =
        crate::output_path::claim_artifact_path(&root, Path::new("../receipt.json"), "receipt")
            .expect_err("parent traversal rejected");
    assert!(traversal.contains("must stay inside the package root"));

    for path in [
        "target/ultragoal-test-receipts/receipt.json",
        "artifacts/review/receipt.json",
        ".harness/receipt.json",
        "receipt.json",
    ] {
        let err = crate::output_path::claim_artifact_path(&root, Path::new(path), "receipt")
            .expect_err("ungoverned root-relative claim output rejected");
        assert!(err.contains("governed claim artifact root"), "{err}");
        assert!(err.contains("external debug only"), "{err}");
    }

    let current_dir = crate::output_path::claim_artifact_path(
        &root,
        Path::new("./validation_artifacts/observability/receipt.json"),
        "receipt",
    )
    .expect_err("current-dir segments are not normalized claim paths");
    assert!(current_dir.contains("must stay inside the package root"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn literal_claim_artifact_path_accepts_product_owned_constants() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("literal-claim-output");
    fs::create_dir_all(&root).expect("root");

    let path = crate::output_path::literal_claim_artifact_path(
        &root,
        "validation_artifacts/observability/static-receipt.json",
        "static receipt",
    );
    assert_eq!(
        path,
        root.join("validation_artifacts/observability/static-receipt.json")
    );

    let invalid = std::panic::catch_unwind(|| {
        crate::output_path::literal_claim_artifact_path(&root, "/tmp/receipt.json", "bad receipt");
    });
    assert!(invalid.is_err());

    let _ = fs::remove_dir_all(root);
}
