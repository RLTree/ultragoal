use serde_json::json;

#[test]
fn review_target_normalization_removes_detached_proof_surfaces() {
    let manifest = json!({
        "schemas":["schemas/keep.json", "validation_artifacts/coverage/drop.json", {"path":"non-string-schema"}],
        "fixtures":["fixtures/keep.json", "validation_artifacts/ultragoal-audit/drop.json"],
        "authorable_templates":["templates/keep.md"],
        "generated_examples":["examples/generated/keep.json"],
        "resources":["docs/keep.md", "validation_artifacts/harness/drop.json"],
        "skills":[{"path":"skills/keep/SKILL.md"},{"path":"validation_artifacts/cli/drop.json"},{"name":"no-path"}],
        "agents":[{"path":"agents/keep.md"},{"path":"validation_artifacts/standards-gardener/drop.json"},{"name":"no-path"}],
        "schema_catalog":"validation_artifacts/coverage/schema-catalog.json"
    });
    let normalized = crate::package::normalized_manifest_for_review(&manifest);
    assert_eq!(
        normalized["schemas"],
        json!(["schemas/keep.json", {"path":"non-string-schema"}])
    );
    assert_eq!(normalized["fixtures"], json!(["fixtures/keep.json"]));
    assert_eq!(normalized["resources"], json!(["docs/keep.md"]));
    assert_eq!(
        normalized["skills"],
        json!([{"path":"skills/keep/SKILL.md"},{"name":"no-path"}])
    );
    assert_eq!(
        normalized["agents"],
        json!([{"path":"agents/keep.md"},{"name":"no-path"}])
    );
    assert!(normalized.get("schema_catalog").is_none());

    let non_arrays = json!({
        "schemas": {"not": "array"},
        "skills": {"not": "array"},
        "schema_catalog": "schemas/catalog.json"
    });
    let normalized = crate::package::normalized_manifest_for_review(&non_arrays);
    assert_eq!(normalized["schemas"], json!({"not":"array"}));
    assert_eq!(normalized["skills"], json!({"not":"array"}));
    assert_eq!(normalized["schema_catalog"], json!("schemas/catalog.json"));
}

#[test]
fn review_payload_rejects_missing_manifest_paths() {
    let root = crate::self_tests::boundaries::support::temp_root("review-payload-missing");
    std::fs::create_dir_all(root.join("docs")).expect("docs");
    assert!(
        crate::package::review_payload(&root, "docs/missing.md")
            .expect_err("missing review payload")
            .contains("review target manifest path is missing")
    );
    std::fs::remove_dir_all(root).expect("cleanup review payload");
}

#[test]
fn review_payload_normalizes_plugin_manifest_and_reads_regular_files() {
    let root = crate::self_tests::boundaries::support::temp_root("review-payload-normalize");
    std::fs::create_dir_all(root.join("docs")).expect("docs");
    std::fs::write(root.join("docs/visible.md"), "visible").expect("visible");
    std::fs::write(
        root.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&json!({
            "resources": ["docs/visible.md", "validation_artifacts/coverage/drop.json"],
            "schemas": ["schemas/keep.schema.json"],
            "schema_catalog": "validation_artifacts/coverage/schema-catalog.json"
        }))
        .expect("manifest"),
    )
    .expect("manifest");

    let payload =
        crate::package::review_payload(&root, "plugin-manifest-draft.json").expect("payload");
    let normalized: serde_json::Value =
        serde_json::from_slice(&payload).expect("normalized manifest json");
    assert_eq!(normalized["resources"], json!(["docs/visible.md"]));
    assert!(normalized.get("schema_catalog").is_none());

    let file = crate::package::review_payload(&root, "docs/visible.md").expect("file payload");
    assert_eq!(file, b"visible");
    assert!(
        crate::package::review_payload(&root, "../escape.md")
            .expect_err("escape rejected")
            .contains("package path escapes package root")
    );
    std::fs::remove_dir_all(root).expect("cleanup review payload normalize");
}

#[cfg(unix)]
#[test]
fn review_payload_rejects_hard_linked_manifest_entries() {
    let root = crate::self_tests::boundaries::support::temp_root("review-payload-hardlink");
    std::fs::create_dir_all(root.join("docs")).expect("docs");
    std::fs::write(root.join("docs/source.md"), "source").expect("source");
    std::fs::hard_link(root.join("docs/source.md"), root.join("docs/hard.md")).expect("hard link");
    let err = crate::package::review_payload(&root, "docs/hard.md")
        .expect_err("hard-linked review payload rejected");
    assert!(err.contains("review payload read failed"), "{err}");
    assert!(err.contains("hard-linked file rejected"), "{err}");
    std::fs::remove_dir_all(root).expect("cleanup hardlink review payload");
}

#[test]
fn build_review_target_receipt_hashes_included_paths_and_validator_anchor() {
    let root = crate::self_tests::boundaries::support::temp_root("review-target-receipt");
    std::fs::create_dir_all(root.join("docs")).expect("docs");
    std::fs::create_dir_all(root.join("validation_artifacts/ultragoal-audit"))
        .expect("audit artifacts");
    std::fs::write(root.join("docs/file.md"), "review me").expect("doc");
    std::fs::write(
        root.join("validation_artifacts/ultragoal-audit/validator-receipt.json"),
        "{}",
    )
    .expect("validator receipt");
    std::fs::write(
        root.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&json!({
            "resources":["docs/file.md"],
            "schemas":["validation_artifacts/coverage/drop.schema.json"],
            "schema_catalog":"validation_artifacts/coverage/schema-catalog.json"
        }))
        .expect("manifest bytes"),
    )
    .expect("manifest");

    let receipt =
        crate::package::build_review_target_receipt(&root).expect("review target receipt");
    assert_eq!(receipt["status"], "pass");
    assert_eq!(receipt["included_path_count"], 1);
    assert_eq!(receipt["excluded_path_count"], 2);
    assert!(
        receipt["package_digest"]
            .as_str()
            .unwrap_or("")
            .starts_with("sha256:")
    );
    assert!(
        receipt["review_target_digest"]
            .as_str()
            .unwrap_or("")
            .starts_with("sha256:")
    );
    assert_eq!(
        receipt["validator_receipt"]["path"],
        "validation_artifacts/ultragoal-audit/validator-receipt.json"
    );
    std::fs::remove_dir_all(root).expect("cleanup review target receipt");
}

#[test]
fn build_review_target_receipt_rejects_unclosed_manifest_paths() {
    let root = crate::self_tests::boundaries::support::temp_root("review-target-unclosed");
    std::fs::create_dir_all(root.join("docs")).expect("docs");
    std::fs::write(
        root.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&json!({
            "resources":["docs/missing.md", "../escape.md"]
        }))
        .expect("manifest bytes"),
    )
    .expect("manifest");
    let err = crate::package::build_review_target_receipt(&root)
        .expect_err("unclosed review target rejected");
    assert!(err.contains("review target is not closed"), "{err}");
    assert!(err.contains("invalid="), "{err}");
    assert!(err.contains("missing="), "{err}");
    std::fs::remove_dir_all(root).expect("cleanup unclosed review target");
}
