use crate::audit::contract::Failure;
use serde_json::{Value, json};
use std::path::Path;

fn errors(out: &[Failure]) -> Vec<&str> {
    out.iter().map(|failure| failure.error.as_str()).collect()
}

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn included_claim(id: &str, title: &str) -> Value {
    json!({
        "id": id,
        "title": title,
        "claim_ceiling_effect": "included"
    })
}

#[test]
fn product_fitness_rejects_substitutes_and_stale_receipt_bindings() {
    let root = crate::self_tests::boundaries::support::temp_root("product-fitness-claim");
    let mut claim = included_claim(
        "PF",
        "Product success via reviewer approved install success smoke test tests passed package publication first use feature delivered",
    );
    claim["requires_product_cohesion"] = json!(true);
    claim["evidence"] = json!([]);
    let mut out = Vec::<Failure>::new();
    crate::claim_semantics::product::fitness::check(&claim, &root, &mut out);
    let got = errors(&out);
    for expected in [
        "product_fitness_quality_in_use_receipt_missing",
        "product_fitness_reviewer_substituted_for_user_evidence",
        "product_fitness_install_substituted_for_success",
        "product_fitness_smoke_test_substituted_for_success",
        "product_fitness_test_pass_substituted_for_user_value",
        "product_fitness_package_publication_substituted_for_product_success",
        "product_fitness_first_use_substituted_for_success",
        "product_fitness_output_substituted_for_outcome",
    ] {
        assert!(got.contains(&expected), "{expected}: {got:?}");
    }

    claim["evidence"] = json!([{
        "kind":"product::fitness::receipt",
        "surface":"product::fitness",
        "path":"../escape.json",
        "digest":crate::self_tests::boundaries::support::sha('0')
    }]);
    out.clear();
    crate::claim_semantics::product::fitness::check(&claim, &root, &mut out);
    assert!(errors(&out).contains(&"product_fitness_receipt_path_invalid"));

    let receipt_path = root.join("receipts/product-fitness.json");
    write_json(
        &receipt_path,
        &json!({"claim":{"id":"OTHER","repeated_use_claimed":false}}),
    );
    let mut repeated = included_claim("PF", "This is a daily driver with repeated use retention");
    repeated["requires_product_cohesion"] = json!(true);
    repeated["evidence"] = json!([{
        "kind":"product::fitness::receipt",
        "surface":"product::fitness",
        "path":"receipts/product-fitness.json",
        "digest":crate::digest::file(&receipt_path).expect("receipt digest")
    }]);
    out.clear();
    crate::claim_semantics::product::fitness::check(&repeated, &root, &mut out);
    let got = errors(&out);
    assert!(got.contains(&"product_fitness_receipt_wrong_claim_id"));
    assert!(got.contains(&"product_fitness_daily_driver_overclaim"));
    std::fs::remove_dir_all(root).expect("cleanup product fitness");
}

#[test]
fn product_cohesion_rejects_weak_applicability_and_missing_product_evidence() {
    let cases = [
        (
            json!({
                "id":"PC1",
                "title":"Customer facing user journey",
                "claim_ceiling_effect":"included",
                "product_applicability":{"surface_classification":"internal_non_product"}
            }),
            "product_applicability_contradicts_claim_text",
        ),
        (
            json!({
                "id":"PC2",
                "title":"Internal control",
                "claim_ceiling_effect":"included",
                "product_applicability":{"user_facing":true}
            }),
            "product_applicability_requires_cohesion",
        ),
        (
            included_claim("PC3", "Release ready user journey"),
            "product_claim_text_requires_cohesion",
        ),
        (
            json!({
                "id":"PC4",
                "title":"UI claim",
                "claim_ceiling_effect":"included",
                "claim_surface":"ui_browser",
                "product_cohesion_waiver":{"reason":"no"}
            }),
            "product_cohesion_waiver_on_consumer_ui_claim",
        ),
        (
            json!({
                "id":"PC5",
                "title":"UI claim",
                "claim_ceiling_effect":"included",
                "claim_surface":"ui_browser"
            }),
            "product_cohesion_applicability_missing",
        ),
        (
            json!({
                "id":"PC6",
                "title":"Feature done",
                "claim_ceiling_effect":"included",
                "claim_kind":"feature_completion"
            }),
            "product_applicability_missing",
        ),
        (
            json!({
                "id":"PC7",
                "title":"Feature done",
                "claim_ceiling_effect":"included",
                "claim_kind":"feature_completion",
                "product_applicability":{"surface_classification":"unknown"}
            }),
            "product_non_product_classification_missing",
        ),
        (
            json!({
                "id":"PC8",
                "title":"Feature done",
                "claim_ceiling_effect":"included",
                "claim_kind":"feature_completion",
                "product_applicability":{"user_facing":true,"surface_classification":"internal_non_product"}
            }),
            "product_applicability_conflicting_classification",
        ),
    ];
    for (claim, expected) in cases {
        let failures = crate::claim_semantics::product::cohesion::product_cohesion_failures(&claim);
        assert!(
            errors(&failures).contains(&expected),
            "{expected}: {failures:?}"
        );
    }

    let mut explicit = included_claim("PC9", "Product cohesion claim");
    explicit["requires_product_cohesion"] = json!(true);
    explicit["evidence"] = json!([]);
    let failures = crate::claim_semantics::product::cohesion::product_cohesion_failures(&explicit);
    let got = errors(&failures);
    assert!(got.contains(&"product_cohesion_receipt_missing"));
    assert!(got.contains(&"product_ui_journey_evidence_missing"));
}
