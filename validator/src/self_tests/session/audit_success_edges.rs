use serde_json::{Value, json};
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn errors(out: &[crate::audit::contract::Failure]) -> Vec<&str> {
    out.iter().map(|failure| failure.error.as_str()).collect()
}

#[test]
fn audit_paths_cover_matching_session_and_namespace_success() {
    let root =
        crate::self_tests::boundaries::support::temp_root("session_and_review-audit-success");
    std::fs::create_dir_all(root.join(".codex-plugin")).expect("plugin dir");
    std::fs::create_dir_all(root.join("validation_artifacts/harness")).expect("harness dir");
    std::fs::write(
        root.join("plugin-manifest-draft.json"),
        "{\"version\":\"0.0.11\"}",
    )
    .expect("manifest");
    std::fs::write(
        root.join(".codex-plugin/plugin.json"),
        "{\"version\":\"0.0.11\"}",
    )
    .expect("plugin");
    write_json(
        &root.join("validation_artifacts/harness/session-packet.json"),
        &json!({"packet":"ok"}),
    );
    write_json(
        &root.join("validation_artifacts/harness/session-log-hardening-receipt.json"),
        &crate::self_tests::session::hardening::complete_receipt_for_root(&root),
    );
    let store = crate::schema_catalog::load(&crate::self_tests::boundaries::support::repo_root());
    let session = crate::audit::session_log_hardening::package_failures(&root, &store);
    let session_text = session.join("\n");
    assert!(!session_text.contains("candidate_version_mismatch"));
    assert!(!session_text.contains("version_unreadable"));

    let failures = crate::audit::source_obligations::value_failures(&json!({"obligations":[{
        "id":"namespace-progressive-disclosure",
        "obligation":"namespace progressive validator red fixture receipt",
        "package_surface":"namespace progressive validator red package surface",
        "enforcement_disposition":"deterministic namespace progressive validator red",
        "missing_validation_fixture_or_receipt":"red fixture and receipt",
        "claim_ceiling_impact":"blocks completion claims when namespace progressive validator red proof fails",
        "coverage_status":"deterministic coverage"
    }]}));
    let failure_text = failures.join("\n");
    assert!(
        !failure_text.contains("namespace-progressive-disclosure:"),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup session_and_review audit success");
}

#[test]
fn claim_paths_cover_dogfood_lane_and_package_boundaries() {
    let root =
        crate::self_tests::boundaries::support::temp_root("session_and_review-claim-boundaries");
    write_json(
        &root.join("schemas/schema-catalog.json"),
        &json!({"schemas":[{"id":"dogfood-receipt.schema.json","path":"schemas/dogfood-receipt.schema.json"}]}),
    );
    write_json(
        &root.join("schemas/dogfood-receipt.schema.json"),
        &json!({"$id":"dogfood-receipt.schema.json","type":"object"}),
    );
    let receipt_path = root.join("receipts/dogfood.json");
    write_json(
        &receipt_path,
        &json!({"claim_id":"DOG","lanes":[],"root_owed_follow_up":{"status":"open"}}),
    );
    let receipt_digest = crate::digest::file(&receipt_path).expect("dogfood digest");
    let mut dogfood = Vec::new();
    crate::claim_semantics::dogfood_receipt::check(
        &json!({
            "id":"DOG",
            "title":"Real multi lane dogfood run",
            "description":"The plugin completed a real dogfood run with root integration.",
            "status":"pass",
            "claim_ceiling_effect":"included",
            "evidence":[{
                "id":"dog",
                "kind":"dogfood_receipt",
                "surface":"root_integration",
                "path":"receipts/dogfood.json",
                "digest":receipt_digest
            }]
        }),
        &root,
        &mut dogfood,
    );
    assert!(errors(&dogfood).contains(&"dogfood_receipt_invalid"));

    let absolute = format!(
        "{}tmp{}package-proof.json",
        std::path::MAIN_SEPARATOR,
        std::path::MAIN_SEPARATOR
    );
    assert!(
        crate::package::inventory::package_path_error(&root, &absolute)
            .expect("absolute rejected")
            .contains("absolute"),
    );

    let mut overlap = Vec::new();
    let lanes = json!([
        {"owned_paths":["src/module"]},
        {"owned_paths":["src"]}
    ]);
    let lane_rows = lanes.as_array().expect("lanes").clone();
    crate::claim_semantics::lane::root::scope::lane_overlap(&lane_rows, &mut overlap);
    assert!(errors(&overlap).contains(&"active_lane_owned_path_overlap"));
    std::fs::remove_dir_all(root).expect("cleanup session_and_review claim boundaries");
}

#[test]
fn target_symlink_parent_creation_is_explicit() {
    let root =
        crate::self_tests::boundaries::support::temp_root("session_and_review-symlink-parent");
    write_json(
        &root.join("validation_artifacts/product-cohesion/symlink-fixture.json"),
        &json!({
            "schema":"harness-ultragoal.target-fixture-symlink.v1",
            "link_path":"deep/new-parent/link",
            "target_path":"target.txt"
        }),
    );
    let fixture = crate::target_fixtures::materialize_symlink_fixture(&root)
        .expect("symlink materialized")
        .expect("fixture present");
    assert!(fixture.link.parent().expect("link parent").is_dir());
    fixture.cleanup().expect("cleanup symlink");
    std::fs::remove_dir_all(root).expect("cleanup session_and_review symlink parent");
}
