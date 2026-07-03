use serde_json::{Value, json};

fn fixture_receipt() -> Value {
    crate::json_boundary::read_json(
        &crate::self_tests::boundaries::workspace_fixtures::repo_root()
            .join("fixtures/target-repo/valid-product-cohesion/validation_artifacts/product-cohesion/journey-receipt.json"),
    )
    .expect("product cohesion fixture")
}

#[test]
fn product_cohesion_schema_rules_reject_invalid_authority_values() {
    let mut receipt = fixture_receipt();
    receipt["schema"] = json!("wrong");
    receipt["primary_journey"]["steps"][0]["evidence"]["digest"] = json!("not-sha");
    receipt["proof"]["runtime_evidence"] = json!("not-array");
    receipt["human_attention_policy"]["expected_interruption_rate"] = json!("always");
    receipt["human_attention_policy"]["allowed_reasons"] = json!("not-array");
    receipt["review"]["reviewer_authority"]["actor_disjoint"] = json!(false);
    receipt["review"]["signoff_status"] = json!("ship");

    let errors = crate::schema_catalog::product_cohesion_receipt_errors(&receipt);
    for expected in [
        "schema must be product-cohesion-receipt.v1",
        "primary_journey.steps[0].evidence.digest must be sha256",
        "proof.runtime_evidence must be an array",
        "human_attention_policy.expected_interruption_rate invalid",
        "human_attention_policy.allowed_reasons must be an array",
        "review.reviewer_authority.actor_disjoint must be true",
        "review.signoff_status invalid",
    ] {
        assert!(
            errors.iter().any(|error| error == expected),
            "{expected}: {errors:?}"
        );
    }
}
