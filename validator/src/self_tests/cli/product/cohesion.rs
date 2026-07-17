use std::path::{Path, PathBuf};

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("copy target");
    for entry in std::fs::read_dir(from).expect("read fixture dir") {
        let entry = entry.expect("dir entry");
        let dest = to.join(entry.file_name());
        if entry.file_type().expect("file type").is_dir() {
            copy_dir(&entry.path(), &dest);
        } else {
            std::fs::copy(entry.path(), dest).expect("copy fixture file");
        }
    }
}

fn collect_files(root: &Path, rel: &Path, out: &mut Vec<String>) {
    for entry in std::fs::read_dir(root.join(rel)).expect("read fixture tree") {
        let entry = entry.expect("dir entry");
        let child = rel.join(entry.file_name());
        if entry.file_type().expect("file type").is_dir() {
            collect_files(root, &child, out);
        } else {
            out.push(child.to_string_lossy().replace('\\', "/"));
        }
    }
}

fn copied_valid_cohesion_fixture() -> PathBuf {
    let repo = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let target = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "product-cohesion-pass-command",
    );
    copy_dir(&repo.join("fixtures/product-cohesion/valid"), &target);
    std::fs::create_dir_all(target.join("docs/generated")).expect("generated directory");
    std::fs::write(
        target.join("docs/generated/index.md"),
        "# Generated product-cohesion fixture\n",
    )
    .expect("generated cohesion index");
    let generated =
        std::fs::read(target.join("docs/generated/index.md")).expect("generated cohesion index");
    let digest = crate::digest::bytes(&generated)
        .strip_prefix("sha256:")
        .expect("digest prefix")
        .to_string();
    std::fs::create_dir_all(target.join("migration")).expect("migration directory");
    std::fs::write(
        target.join("migration/generated-surface-authority.json"),
        serde_json::to_vec(&serde_json::json!({
            "schema_version": "GeneratedSurfaceAuthority-v2",
            "contract_id": "harness-ultragoal-successor-contract-v2",
            "surfaces": [{
                "disposition": "retained_context",
                "output": "docs/generated/index.md",
                "sha256": digest,
                "reason": "ephemeral product-cohesion fixture context",
                "replacement_targets": ["HCT-INVENTORY"],
                "preserve": true,
                "physical_deletion_authorized": false
            }]
        }))
        .expect("generated authority fixture"),
    )
    .expect("write generated authority fixture");
    let mut resources = Vec::new();
    collect_files(&target, Path::new(""), &mut resources);
    resources.sort();
    std::fs::write(
        target.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&serde_json::json!({"resources": resources})).expect("manifest"),
    )
    .expect("write manifest");
    target
}

#[test]
fn product_cohesion_command_emits_pass_observability_for_valid_artifacts() {
    let root = copied_valid_cohesion_fixture();
    let obs = PathBuf::from("validation_artifacts/observability/product-prove-cohesion.json");
    let code = crate::cli::product::run(
        &root,
        &crate::cli::product::ProductCommand {
            operation: crate::cli::product::ProductOperation::ProductProveCohesion,
            receipt_dir: None,
            observability_receipt: obs.clone(),
        },
    )
    .expect("cohesion command emits telemetry");
    assert_eq!(code, 0);
    let receipt = crate::json_boundary::read_json(&root.join(obs)).expect("receipt");
    assert_eq!(receipt["status"], "pass");
    assert_eq!(receipt["event"]["operation"], "product.prove-cohesion");
    assert_eq!(receipt["event"]["failure_class"], "none");
    assert_eq!(receipt["supported_claims"][0], "product_cohesion");
    std::fs::remove_dir_all(root).expect("cleanup cohesion command");
}
