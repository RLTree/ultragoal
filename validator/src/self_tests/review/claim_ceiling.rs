use serde_json::{Value, json};

pub(crate) fn anchors() -> crate::review::round::anchor::values::AnchorValues {
    crate::review::round::anchor::values::AnchorValues {
        validator_path: "validator.json".into(),
        review_target_path: "review-target.json".into(),
        archive_path: "archive.json".into(),
        validator_digest: crate::self_tests::boundaries::workspace_fixtures::sha('1'),
        review_target_digest: crate::self_tests::boundaries::workspace_fixtures::sha('2'),
        archive_digest: crate::self_tests::boundaries::workspace_fixtures::sha('3'),
        validator_run_id: "run".into(),
        package_digest: crate::self_tests::boundaries::workspace_fixtures::sha('4'),
        source_errors: Vec::new(),
        materiality_anchors: Vec::new(),
    }
}

fn entries(ids: &[&str], status: &str) -> Vec<Value> {
    ids.iter()
        .map(|claim_id| json!({"claim_id":claim_id,"status":status,"rationale":"test"}))
        .collect()
}

const SUPPORTED: &[&str] = &[
    "package_static_fixture_proof",
    "detached_review_target_archive_identity",
];

const UNSUPPORTED: &[&str] = &[
    "plugins_ui_visibility",
    "install_button_success",
    "workspace_public_marketplace_publication",
    "real_multilane_dogfood",
    "production_readiness",
    "external_product_ux_improvement",
    "reviewer_runtime_configuration",
    "custom_agent_runtime_discovery",
];

pub(crate) fn receipt() -> Value {
    json!({"claim_ceiling":{
        "supported":entries(SUPPORTED, "supported"),
        "unsupported":entries(UNSUPPORTED, "unsupported")
    }})
}

fn assessment() -> Value {
    json!({
        "authority":"falsification_only_cannot_raise",
        "not_disproven":entries(SUPPORTED, "supported"),
        "challenged":entries(UNSUPPORTED, "unsupported")
    })
}

fn failures(receipt: &Value, assessment: Value) -> Vec<String> {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let mut out = Vec::new();
    crate::review::round::claim::ceiling::row_authority_errors(
        &root,
        receipt,
        &anchors(),
        &json!({"claim_ceiling_assessment":assessment}),
        "claim-falsifier",
        &mut out,
    );
    out.into_iter().map(|failure| failure.error).collect()
}

#[test]
fn reviewer_claim_assessment_cannot_raise_the_deterministic_ceiling() {
    let mut raised = assessment();
    raised["not_disproven"].as_array_mut().unwrap().push(json!({
        "claim_id":"production_readiness",
        "status":"supported",
        "rationale":"reviewer tried to raise the ceiling"
    }));
    let got = failures(&receipt(), raised);
    assert!(
        got.contains(&"review_round_claim_ceiling_overclaim".to_string()),
        "{got:?}"
    );
}

#[test]
fn reviewer_claim_assessment_requires_lower_only_authority_and_full_challenges() {
    let got = failures(&receipt(), json!({}));
    assert!(
        got.contains(&"review_round_claim_ceiling_authority_invalid".to_string()),
        "{got:?}"
    );

    let mut missing = assessment();
    missing["challenged"].as_array_mut().unwrap().pop();
    let got = failures(&receipt(), missing);
    assert!(
        got.contains(&"review_round_claim_ceiling_mismatch".to_string()),
        "{got:?}"
    );

    let mut duplicate = assessment();
    let first = duplicate["challenged"][0].clone();
    duplicate["challenged"].as_array_mut().unwrap().push(first);
    let got = failures(&receipt(), duplicate);
    assert!(
        got.contains(&"review_round_claim_ceiling_duplicate".to_string()),
        "{got:?}"
    );
}

#[test]
fn deterministic_claim_ceiling_rejects_live_and_product_substitution() {
    let mut overclaim = receipt();
    overclaim["claim_ceiling"]["supported"] =
        entries(&["live_runtime_ui_visibility"], "supported").into();
    let got = failures(&overclaim, assessment());
    assert!(
        got.contains(&"review_round_claim_ceiling_overclaim".to_string()),
        "{got:?}"
    );

    let mut incomplete = receipt();
    incomplete["claim_ceiling"]["unsupported"]
        .as_array_mut()
        .unwrap()
        .pop();
    let got = failures(&incomplete, assessment());
    assert!(
        got.contains(&"review_round_claim_ceiling_missing".to_string()),
        "{got:?}"
    );
}
