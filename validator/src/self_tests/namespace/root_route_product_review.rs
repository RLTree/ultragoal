use serde_json::{Value, json};
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

fn contains(items: &[String], needle: &str) -> bool {
    items.iter().any(|item| item.contains(needle))
}

#[test]
fn stale_review_json_recurses_and_namespace_root_routes_allow_known_files() {
    let stale = crate::audit::text_guards::stale_review_law_value_failures(&json!([
        "all six reviewers",
        {"nested":["GPT-5.4-mini provisional approval", 7]},
        false
    ]));
    assert!(
        stale.iter().any(|row| row
            .contains("materialized-json/1/nested/0:1: stale_review_cadence_in_current_surface")),
        "{stale:?}"
    );

    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("namespace-root-route");
    write_text(&root.join("README.md"), "readme");
    write_text(&root.join("rust-toolchain.toml"), "toolchain");
    let allowed = crate::audit::namespace::law::value_failures(
        &root,
        &json!({"resources":["README.md","rust-toolchain.toml","bad-root-file.md"]}),
    );
    assert!(contains(
        &allowed,
        "namespace_root_clutter_without_route:bad-root-file.md"
    ));
    assert!(!contains(
        &allowed,
        "namespace_root_clutter_without_route:README.md"
    ));
    assert!(!contains(
        &allowed,
        "namespace_root_clutter_without_route:rust-toolchain.toml"
    ));
    std::fs::remove_dir_all(root).expect("cleanup namespace route");
}

#[test]
fn product_fitness_review_requires_receipt_claim_subset_and_current_digest() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("pf-review-subset");
    let receipt_path = root.join("validation_artifacts/harness/product-fitness-receipt.json");
    write_json(
        &receipt_path,
        &json!({"generated_at":"2026-06-26T00:00:00Z"}),
    );
    let digest = crate::digest::file(&receipt_path).expect("pf digest");
    let mut out = Vec::new();
    crate::review::round::product::fitness::disposition_errors(
        &root,
        &json!({
            "agent_type":"harness_product_simplicity_falsifier",
            "product_fitness_required":true,
            "product_fitness_owner":"product_simplicity_falsifier",
            "product_fitness_receipt_digest":digest,
            "product_fitness_claim_ids":["production_readiness"],
            "substitution_rejections_reviewed":[
                "generic_product_simplicity_approval","product::cohesion","install_success",
                "package_publication","first_use","smoke_test","fixture_pass",
                "reviewer_agreement","happy_path","receipt_only"
            ],
            "product_fitness_disposition":{
                "owner_persona":"product_simplicity_falsifier",
                "owner_agent_type":"harness_product_simplicity_falsifier",
                "applies_to_product_impacting_claims":true,
                "claim_ids_reviewed":["production_readiness"],
                "generic_product_simplicity_approval_only":false,
                "substitution_rejection":true,
                "dimensions_checked":[
                    "audience","job","context","outcome","accessibility","cognitive_load",
                    "recovery_burden","continuance","real_use_evidence",
                    "product_success_substitution_rejection"
                ],
                "receipt":{
                    "path":"validation_artifacts/harness/product-fitness-receipt.json",
                    "digest":digest
                },
                "receipt_generated_at":"2026-06-26T00:00:00Z"
            }
        }),
        &json!({
            "claim_ceiling":{"unsupported":[{"claim_id":"release_readiness"}]},
            "materiality_gate":{"claims":["product release readiness"]}
        }),
        "product_simplicity_falsifier",
        &mut out,
    );
    let errors = out
        .into_iter()
        .map(|failure| failure.error)
        .collect::<Vec<_>>();
    assert!(errors.contains(&"review_round_product_fitness_claim_binding_missing".to_string()));
    std::fs::remove_dir_all(root).expect("cleanup pf review subset");
}
