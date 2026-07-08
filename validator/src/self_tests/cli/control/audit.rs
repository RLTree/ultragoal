use serde_json::json;
use std::path::Path;

fn write_json(path: &Path, value: &serde_json::Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn write_text(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, text).expect("write text");
}

#[test]
fn cli_control_plane_audit_rejects_missing_schema_and_invalid_receipts() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("cli-control-plane-audit");
    write_text(
        &root.join("validator/Cargo.toml"),
        "[[bin]]\nname = \"ultragoal\"\n",
    );
    write_text(
        &root.join("validator/src/cli/control/plane/mod.rs"),
        "source",
    );
    write_text(
        &root.join("validator/src/cli/control/plane/types.rs"),
        "types",
    );
    write_json(
        &root.join("schemas/cli-control-plane-receipt.schema.json"),
        &json!({}),
    );
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[
            {"path":"validator/src/cli/control/plane/mod.rs"},
            {"path":"validator/src/cli/control/plane/types.rs"},
            {"path":"schemas/cli-control-plane-receipt.schema.json"}
        ]}),
    );
    write_json(
        &root.join("schemas/schema-catalog.json"),
        &json!({"schemas":[]}),
    );
    write_json(
        &root.join("templates/agent-standards/enforcement.json"),
        &json!({"rows":[]}),
    );
    write_json(
        &root.join("docs/source-obligation-matrix.json"),
        &json!({"obligations":[]}),
    );
    write_json(
        &root.join("docs/foundational-law-traceability.json"),
        &json!({"entries":[]}),
    );
    write_json(&root.join("templates/RED_FIXTURES.json"), &json!([]));
    write_json(
        &root.join("validation_artifacts/cli/update-goal-eligibility.json"),
        &json!({"schema":"wrong","issuer":{"tool":"manual"}}),
    );
    write_json(
        &root.join("validation_artifacts/cli/self-law-receipt.json"),
        &json!({"schema":"wrong","issuer":{"tool":"manual"}}),
    );
    let failures = crate::audit::cli::control_plane::authority::package_failures(&root);
    assert!(
        failures.contains(&"cli_control_plane_schema_catalog_missing_receipt_schema".to_string())
    );
    assert!(
        !failures
            .iter()
            .any(|failure| failure.contains("missing_compatibility_binary")),
        "legacy compatibility binary is no longer required: {failures:?}"
    );
    assert!(failures.iter().any(|failure| failure.contains(
        "validation_artifacts/cli/update-goal-eligibility.json: cli_control_plane_receipt_wrong_schema"
    )));
    assert!(failures.iter().any(|failure| failure.contains(
        "validation_artifacts/cli/self-law-receipt.json: cli_control_plane_receipt_wrong_issuer"
    )));
    std::fs::remove_dir_all(root).expect("cleanup cli control audit");
}

#[test]
fn cli_control_plane_audit_requires_canonical_binary_not_package_name() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "cli-control-plane-package-name-only",
    );
    write_text(
        &root.join("validator/Cargo.toml"),
        "[package]\nname = \"ultragoal\"\n\n[[bin]]\nname = \"other-tool\"\n",
    );

    let failures = crate::audit::cli::control_plane::authority::package_failures(&root);
    assert!(
        failures.contains(&"cli_control_plane_missing_ultragoal_binary".to_string()),
        "package name must not satisfy canonical binary authority: {failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup cli control package-name-only audit");
}

#[test]
fn cli_control_plane_audit_rejects_legacy_validator_binary() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "cli-control-plane-legacy-bin",
    );
    write_text(
        &root.join("validator/Cargo.toml"),
        r#"
[[bin]]
name = "ultragoal"
path = "src/bin/ultragoal.rs"

[[bin]]
name = "ultragoal-validator"
path = "src/bin/ultragoal-validator.rs"
"#,
    );

    let failures = crate::audit::cli::control_plane::authority::package_failures(&root);
    assert!(
        failures.contains(&"cli_control_plane_legacy_validator_binary_present".to_string()),
        "legacy validator binary must not remain an alternate authority: {failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup cli control legacy binary audit");
}
