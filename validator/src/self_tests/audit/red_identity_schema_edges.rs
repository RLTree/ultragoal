use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
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
fn law_surface_red_identity_covers_green_edges() {
    let repo = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let package_failures = crate::audit::law::surface::receipts::package_failures(&repo);
    assert!(
        package_failures.is_empty(),
        "valid law-surface fixtures should stay green: {package_failures:?}"
    );

    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("red-identity-green");
    write_json(
        &root.join("schemas/schema-catalog.json"),
        &json!({"schemas":[
            {"id":"schema-authority-primitives.schema.json","path":"schemas/schema-authority-primitives.schema.json"},
            {"id":"validator-receipt.schema.json","path":"schemas/validator-receipt.schema.json"}
        ]}),
    );
    write_json(
        &root.join("schemas/schema-authority-primitives.schema.json"),
        &json!({"$id":"schema-authority-primitives.schema.json","$defs":{"requiredRedFixtureId":{"enum":["red-one"]}}}),
    );
    write_json(
        &root.join("schemas/validator-receipt.schema.json"),
        &json!({"$id":"validator-receipt.schema.json","properties":{"red_fixtures":{"required":["red-one"]}}}),
    );
    write_json(
        &root.join("fixtures/red/one.json"),
        &json!({"expected_failure":{"check_id":"schema-valid","error":"x"}}),
    );
    let digest = crate::digest::file(&root.join("fixtures/red/one.json")).expect("red digest");
    write_json(
        &root.join("templates/RED_FIXTURES.json"),
        &json!([{
            "id":"red-one",
            "packet_path":"fixtures/red/one.json",
            "packet_digest":digest,
            "expected_failure":{"check_id":"schema-valid","error":"x"}
        }]),
    );
    let store = crate::schema_catalog::load(&root);
    let mut failures = BTreeMap::new();
    crate::audit::red::catalog::check(&root, &store, &mut failures);
    assert!(failures.is_empty(), "{failures:?}");

    std::fs::remove_dir_all(root).expect("cleanup red identity green");
}

#[test]
fn source_obligation_text_guard_and_session_edges_cover_current_paths() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("audit-current-edges");
    let source_cards = json!([{
        "id":"fresh",
        "retrieval_receipt":{"status":"refreshed"},
        "notes":"current implementation",
        "cited_claims":[]
    }]);
    write_json(&root.join("docs/source-cards.json"), &source_cards);
    assert!(crate::audit::text_guards::source_card_freshness_failures(&root).is_empty());
    assert!(
        crate::audit::text_guards::moving_value_value_failures(&json!({
            "safe":"checks have no slash count here",
            "nested":[7]
        }))
        .is_empty()
    );

    let failures = crate::audit::source_obligations::value_failures(&json!({"obligations":[{
        "id":"namespace-progressive-disclosure",
        "enforcement_disposition":"deterministic namespace progressive validator red",
        "missing_validation_fixture_or_receipt":"red fixture and receipt",
        "claim_ceiling_impact":"blocks"
    }]}));
    assert!(
        !failures
            .iter()
            .any(|failure| failure.starts_with("namespace-progressive-disclosure:")),
        "{failures:?}"
    );

    std::fs::create_dir_all(root.join("validation_artifacts/harness")).expect("harness");
    std::fs::create_dir_all(root.join(".codex-plugin")).expect("plugin");
    write_text(
        &root.join("plugin-manifest-draft.json"),
        "{\"version\":\"0.0.11\"}",
    );
    write_text(
        &root.join(".codex-plugin/plugin.json"),
        "{\"version\":\"0.0.10\"}",
    );
    let store = crate::schema_catalog::load(
        &crate::self_tests::boundaries::workspace_fixtures::repo_root(),
    );
    write_json(
        &root.join("validation_artifacts/harness/session-log-hardening-receipt.json"),
        &crate::self_tests::session::hardening::complete_receipt(),
    );
    let session = crate::audit::session_log_hardening::package_failures(&root, &store);
    assert!(
        session
            .iter()
            .any(|failure| failure.starts_with("session_log_hardening_version_unreadable")),
        "{session:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup audit current edges");
}

#[test]
fn schema_max_items_are_exercised() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("schema-symlink-edges");
    write_json(
        &root.join("schemas/schema-catalog.json"),
        &json!({"schemas":[{"id":"max.schema.json","path":"schemas/max.schema.json"}]}),
    );
    write_json(
        &root.join("schemas/max.schema.json"),
        &json!({"$id":"max.schema.json","type":"array","maxItems":1}),
    );
    let store = crate::schema_catalog::load(&root);
    let errors = crate::schema_catalog::schema_errors(&store, "max.schema.json", &json!([1, 2]));
    assert!(
        errors.iter().any(|error| error.contains("maxItems")),
        "{errors:?}"
    );

    std::fs::remove_dir_all(root).expect("cleanup schema edges");
}
