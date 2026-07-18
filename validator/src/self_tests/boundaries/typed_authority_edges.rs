use serde_json::json;
use std::io::{Error as IoError, Write};

fn write_text(path: &std::path::Path, text: &str) {
    let parent = path.parent().expect("test path has parent");
    std::fs::create_dir_all(parent).expect("parent");
    std::fs::write(path, text).expect("write text");
}

struct AlwaysFailWrite;

impl Write for AlwaysFailWrite {
    fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
        Err(IoError::other("forced write failure"))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn boundary_failures_are_behavioral() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("typed_authority-boundaries");
    std::fs::create_dir_all(&root).expect("root");

    let mut sink = AlwaysFailWrite;
    assert!(crate::archive::zip::write_u16(&mut sink, 1).is_err());
    assert!(crate::archive::zip::write_u32(&mut sink, 1).is_err());
    sink.flush().expect("flush succeeds");

    let manifest = json!({"resources":["hard.json"]});
    write_text(&root.join("source.json"), "{}");
    std::fs::hard_link(root.join("source.json"), root.join("hard.json")).expect("hard link");
    write_text(
        &root.join("plugin-manifest-draft.json"),
        &manifest.to_string(),
    );
    let digest_err = crate::package::inventory::package_digest(&root)
        .expect_err("hard-linked package input is rejected");
    assert!(digest_err.contains("hard-linked"));

    let nonexistent = root.join("no-such-file.txt");
    let read_err = crate::digest::read_file_bytes(&nonexistent)
        .expect_err("missing file metadata failure is explicit");
    assert!(read_err.contains("metadata failed"));

    let missing_root = root.join("missing-root");
    let item = json!({"path":"proof.json","digest":crate::self_tests::boundaries::workspace_fixtures::sha('a')});
    let package_err = crate::package::artifact::refs::ArtifactRef::from_object(&item, "proof")
        .and_then(|artifact| artifact.validate(&missing_root, "proof"))
        .expect_err("missing package root is rejected");
    assert!(package_err.contains("package root unavailable"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn law_and_text_boundaries_hit_negative_edges() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("typed_authority-law");
    std::fs::create_dir_all(root.join("fixtures/law-surfaces/valid")).expect("fixtures");
    write_text(
        &root.join("fixtures/law-surfaces/valid/runtime-tool-identity-receipt.json"),
        "{}",
    );
    let law_failures = crate::audit::law::surface::receipts::package_failures(&root);
    assert!(
        law_failures
            .iter()
            .any(|failure| failure.contains("runtime_tool_identity"))
    );

    let cards = json!([
        {
            "id": "stale-current-source",
            "retrieval_receipt": {"status": "not_refreshed"},
            "notes": "current implementation source-backed claim",
            "cited_claims": [
                {"source_support": "live implementation is claimed current"}
            ]
        }
    ]);
    let source_failures = crate::audit::text_guards::source_card_value_failures(&cards);
    assert_eq!(
        source_failures,
        vec!["stale-current-source: source_card_not_refreshed_overclaim"]
    );

    let coverage_file = root.join("coverage.txt");
    write_text(&coverage_file, "coverage");
    let coverage_manifest = json!({
        "changed_file_coupling_policy": {
            "changed_files": ["coverage.txt"]
        }
    });
    assert!(
        crate::audit::coverage::scope::digests::changed_files_digest(&root, &coverage_manifest)
            .expect("changed digest")
            .starts_with("sha256:")
    );
    std::fs::hard_link(&coverage_file, root.join("coverage-hardlink.txt")).expect("hardlink");
    let hardlink_manifest = json!({
        "changed_file_coupling_policy": {
            "changed_files": ["coverage-hardlink.txt"]
        }
    });
    let digest_err =
        crate::audit::coverage::scope::digests::changed_files_digest(&root, &hardlink_manifest)
            .expect_err("hard-linked changed-file input is rejected");
    assert!(digest_err.contains("coverage digest read failed"));

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn cli_law_and_audit_default_paths_are_exercised() {
    let repo = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let cli_failures = crate::audit::cli::control_plane::authority::package_failures(&repo);
    let cli_failure_text = cli_failures.join("\n");
    assert!(!cli_failure_text.contains("missing_standards_row"));
    assert!(!cli_failure_text.contains("missing_foundational_trace"));

    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "typed_authority-audit-default",
    );
    std::fs::create_dir_all(&root).expect("root");
    let receipt = root.join("out/validator-receipt.json");
    let result = crate::audit::run(crate::audit::AuditOptions {
        root: root.clone(),
        receipt,
        red_report: None,
        target_repo: None,
        mode: "retrofit".to_string(),
        require_observability: false,
        require_product_cohesion: false,
        jobs: None,
        command_text: "ultragoal source audit".to_string(),
    });
    assert!(matches!(result, Err(_) | Ok(1)), "{result:?}");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn remaining_typed_boundary_edges_are_enforced() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "typed_authority-remaining-boundaries",
    );
    std::fs::create_dir_all(root.join("schemas")).expect("schemas");
    write_text(
        &root.join("schemas/one.schema.json"),
        r#"{"$id":"one.schema.json","oneOf":[{"type":"number"},{"minimum":1}]}"#,
    );
    write_text(
        &root.join("schemas/schema-catalog.json"),
        r#"{"schemas":[{"id":"one.schema.json","path":"schemas/one.schema.json"}]}"#,
    );
    let store = crate::schema_catalog::load(&root);
    let schema_errors = crate::schema_catalog::schema_errors(&store, "one.schema.json", &json!(2));
    assert!(
        schema_errors
            .iter()
            .any(|err| err.contains("oneOf mismatch")),
        "{schema_errors:?}"
    );

    write_text(&root.join("actual.txt"), "actual");
    let inventory_failures = crate::package::inventory::closure::inventory_closure_failures(
        &root,
        &json!({"resources":["missing.txt","missing.txt"]}),
    );
    let inventory = inventory_failures.join("\n");
    assert!(inventory.contains("missing="), "{inventory}");
    assert!(inventory.contains("duplicates="), "{inventory}");

    let _ = std::fs::remove_dir_all(root);
}
