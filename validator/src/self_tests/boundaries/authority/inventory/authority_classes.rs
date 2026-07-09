use serde_json::json;

#[test]
fn authority_surface_inventory_discovers_required_authority_classes() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("authority-class-inventory");
    let package_paths = [
        "validator/src/cli/package/digest.rs",
        "schemas/package-surface-audit-receipt.schema.json",
        "fixtures/mandatory-law-surfaces/valid/typed-records-over-prose.json",
        "fixtures/red/typed-records-over-prose-raw-path-authority-rejected-red.json",
        "templates/agent-standards/01-namespace-and-progressive-disclosure.md",
        "docs/source-obligation-matrix.json",
        "docs/source-obligation-matrix.md",
        "docs/foundational-law-traceability.json",
        "docs/generated/observability/command-inventory.json",
        "plugin-manifest-draft.json",
        "docs/plugin-cohesion-manifest.json",
        ".codex-plugin/plugin.json",
        ".codex/automations/ultragoal-orchestrator/automation.toml",
        ".codex/automations/ultragoal-orchestrator/transition-receipt.json",
    ];
    for rel in package_paths {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().expect("surface parent")).expect("surface dir");
        let body = if rel == "validator/src/cli/package/digest.rs" {
            "pub(crate) fn run() { let claim_impact = \"source_local\"; let blocked_claims = [\"readiness\"]; }"
        } else {
            "{}"
        };
        std::fs::write(&path, body).expect("surface file");
    }
    let runtime_receipt = "validation_artifacts/observability/package-digest.json";
    std::fs::create_dir_all(root.join("validation_artifacts/observability"))
        .expect("runtime receipt dir");
    std::fs::write(root.join(runtime_receipt), "{}").expect("runtime receipt");
    std::fs::write(
        root.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&json!({ "resources": package_paths })).expect("manifest json"),
    )
    .expect("manifest");

    let inventory = crate::audit::law::authority_surfaces::foundational_surface_inventory(&root);
    for (role, rel, required) in required_authority_rows(runtime_receipt) {
        assert!(
            available_row(&inventory, role, rel, required),
            "missing {role}:{rel}: {inventory}"
        );
    }
    std::fs::remove_dir_all(root).expect("cleanup authority class inventory");
}

#[test]
fn authority_surface_inventory_blocks_unlisted_package_surfaces() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "authority-package-surface-gap",
    );
    let rel = "templates/agent-standards/01-namespace-and-progressive-disclosure.md";
    std::fs::create_dir_all(root.join("templates/agent-standards")).expect("standards dir");
    std::fs::write(root.join(rel), "namespace law").expect("standards file");
    std::fs::write(
        root.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&json!({ "resources": [] })).expect("manifest json"),
    )
    .expect("manifest");

    let inventory = crate::audit::law::authority_surfaces::foundational_surface_inventory(&root);
    assert!(blocked_standards_row(&inventory, rel), "{inventory}");
    std::fs::remove_dir_all(root).expect("cleanup package surface gap");
}

fn required_authority_rows(runtime_receipt: &str) -> Vec<(&'static str, &str, bool)> {
    vec![
        ("source", "validator/src/cli/package/digest.rs", true),
        (
            "schema",
            "schemas/package-surface-audit-receipt.schema.json",
            true,
        ),
        (
            "receipt_schema",
            "schemas/package-surface-audit-receipt.schema.json",
            true,
        ),
        (
            "valid_fixture",
            "fixtures/mandatory-law-surfaces/valid/typed-records-over-prose.json",
            true,
        ),
        (
            "red_fixture",
            "fixtures/red/typed-records-over-prose-raw-path-authority-rejected-red.json",
            true,
        ),
        (
            "standards",
            "templates/agent-standards/01-namespace-and-progressive-disclosure.md",
            true,
        ),
        (
            "source_obligation",
            "docs/source-obligation-matrix.json",
            true,
        ),
        (
            "source_obligation",
            "docs/source-obligation-matrix.md",
            true,
        ),
        (
            "foundational_trace",
            "docs/foundational-law-traceability.json",
            true,
        ),
        (
            "generated_artifact",
            "docs/generated/observability/command-inventory.json",
            true,
        ),
        ("claim_guard", "validator/src/cli/package/digest.rs", true),
        (
            "orchestration_authority",
            ".codex/automations/ultragoal-orchestrator/automation.toml",
            true,
        ),
        (
            "orchestration_authority",
            ".codex/automations/ultragoal-orchestrator/transition-receipt.json",
            true,
        ),
        ("runtime_receipt", runtime_receipt, false),
    ]
}

fn available_row(inventory: &serde_json::Value, role: &str, rel: &str, required: bool) -> bool {
    inventory_rows(inventory).iter().any(|row| {
        row.get("role").and_then(serde_json::Value::as_str) == Some(role)
            && row.get("path").and_then(serde_json::Value::as_str) == Some(rel)
            && row
                .get("package_inventory_required")
                .and_then(serde_json::Value::as_bool)
                == Some(required)
            && row.get("surface_state").and_then(serde_json::Value::as_str) == Some("available")
    })
}

fn blocked_standards_row(inventory: &serde_json::Value, rel: &str) -> bool {
    inventory_rows(inventory).iter().any(|row| {
        row.get("role").and_then(serde_json::Value::as_str) == Some("standards")
            && row.get("path").and_then(serde_json::Value::as_str) == Some(rel)
            && row
                .get("listed_in_package_inventory")
                .and_then(serde_json::Value::as_bool)
                == Some(false)
            && row.get("surface_state").and_then(serde_json::Value::as_str) == Some("blocked")
    })
}

fn inventory_rows(inventory: &serde_json::Value) -> &[serde_json::Value] {
    inventory
        .get("rows")
        .and_then(serde_json::Value::as_array)
        .expect("rows")
}
