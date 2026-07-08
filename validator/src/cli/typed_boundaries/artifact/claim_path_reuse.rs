use serde_json::json;

fn minimal_root(label: &str) -> std::path::PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    std::fs::create_dir_all(&root).expect("root");
    let manifest = root.join("plugin-manifest-draft.json");
    std::fs::write(
        manifest,
        serde_json::to_vec(&json!({
            "name": "typed-boundaries-artifact-test",
            "version": "0.0.0",
            "resources": []
        }))
        .expect("manifest json"),
    )
    .expect("manifest");
    root
}

#[cfg(unix)]
#[test]
fn reused_claim_artifact_rejects_symlinked_parent() {
    use std::os::unix::fs::symlink;

    let root = minimal_root("typed-boundaries-artifact-symlink-parent");
    let receipt = std::path::PathBuf::from("validation_artifacts/typed-boundaries/artifacts.json");
    let first_exit = super::super::run(
        &root,
        &super::super::TypedBoundariesCommand {
            receipt: receipt.clone(),
            jobs: Some(1),
        },
    )
    .expect("initial artifact materialization");
    assert_eq!(first_exit, 1);

    let artifact_path = root.join(super::super::PACKAGE_SURFACE_INVENTORY_REL);
    let artifact_payload = std::fs::read(&artifact_path).expect("artifact payload");
    let linked_parent = artifact_path.parent().expect("artifact parent");
    let outside_parent = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "typed-boundaries-artifact-outside-parent",
    );
    std::fs::create_dir_all(&outside_parent).expect("outside parent");
    std::fs::write(
        outside_parent.join(artifact_path.file_name().expect("artifact filename")),
        artifact_payload,
    )
    .expect("outside artifact payload");
    std::fs::remove_dir_all(linked_parent).expect("remove package artifact parent");
    symlink(&outside_parent, linked_parent).expect("symlink package artifact parent");

    let err = super::super::run(
        &root,
        &super::super::TypedBoundariesCommand {
            receipt,
            jobs: Some(1),
        },
    )
    .expect_err("symlinked parent rejected before artifact reuse");
    assert!(err.contains("output path uses symlink"), "{err}");
    std::fs::remove_dir_all(root).expect("cleanup root");
    std::fs::remove_dir_all(outside_parent).expect("cleanup outside");
}
