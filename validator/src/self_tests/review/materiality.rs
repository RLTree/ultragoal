use serde_json::{Value, json};

fn errors(value: Value) -> Vec<String> {
    crate::review::materiality::value_failures(&value)
}

fn contains(errors: &[String], needle: &str) -> bool {
    errors.iter().any(|error| error == needle)
}

fn ceiling() -> Value {
    json!({"supported":[],"unsupported":["live"],"blocked":["release"]})
}

fn gates() -> Vec<&'static str> {
    vec![
        "anchor_existence_and_digest",
        "validator_receipt_status",
        "reviewer_registry_model_persona_exposure",
        "stale_receipt_check",
        "package_private_artifact_hygiene",
        "readiness_validator",
        "claim_ceiling_check",
        "proof_surface_substitution_check",
    ]
}

#[test]
fn materiality_value_failures_cover_all_decision_shapes() {
    assert!(contains(
        &errors(json!({"decision":"MAYBE"})),
        "materiality_decision_invalid"
    ));

    let missing_gate = errors(json!({
        "decision":"FULL_SCOPE_MATERIAL_REVIEW_REQUIRED",
        "deterministic_gates_required":["anchor_existence_and_digest"],
        "deterministic_gates_run":[],
        "claim_ceiling":ceiling(),
        "reviewers_required":[],
        "output_may_be_used_for_material_signoff":false,
        "anchors_checked":[],
        "reviewer_registry_evidence":[],
        "material_triggers":[]
    }));
    assert!(contains(&missing_gate, "materiality_required_gate_missing"));
    assert!(contains(&missing_gate, "materiality_required_gate_not_run"));
    assert!(contains(
        &missing_gate,
        "materiality_full_scope_reviewer_set_missing"
    ));
    assert!(contains(
        &missing_gate,
        "materiality_full_scope_not_signoff_capable"
    ));
    assert!(contains(
        &missing_gate,
        "materiality_anchor_evidence_missing"
    ));
    assert!(contains(
        &missing_gate,
        "materiality_reviewer_registry_evidence_missing"
    ));
    assert!(contains(&missing_gate, "materiality_trigger_missing"));

    let delta = errors(json!({
        "decision":"DELTA_REVIEW_ALLOWED",
        "deterministic_gates_required":gates(),
        "deterministic_gates_run":gates(),
        "claim_ceiling":ceiling(),
        "reviewers_required":["a","b"],
        "output_may_be_used_for_material_signoff":true
    }));
    assert!(contains(&delta, "materiality_delta_used_for_signoff"));
    assert!(contains(
        &delta,
        "materiality_delta_reviewer_scope_too_broad"
    ));

    let advisory = errors(json!({
        "decision":"ADVISORY_REVIEW_ALLOWED",
        "deterministic_gates_required":gates(),
        "deterministic_gates_run":gates(),
        "claim_ceiling":{"supported":["release"],"unsupported":[],"blocked":[]},
        "output_may_be_used_for_material_signoff":true
    }));
    assert!(contains(&advisory, "materiality_advisory_used_for_signoff"));
    assert!(contains(
        &advisory,
        "materiality_advisory_overclaims_support"
    ));

    let blocked = errors(json!({
        "decision":"BLOCKED_BEFORE_REVIEW",
        "deterministic_gates_required":gates(),
        "deterministic_gates_run":gates(),
        "claim_ceiling":ceiling(),
        "reviewers_required":["contract_claim_falsifier"],
        "validator_repairs_recommended":[]
    }));
    assert!(contains(&blocked, "materiality_blocked_launches_reviewers"));
    assert!(contains(&blocked, "materiality_blocked_without_repair"));
}

#[test]
fn materiality_claim_ceiling_is_required_for_valid_decisions() {
    let result = errors(json!({
        "decision":"ADVISORY_REVIEW_ALLOWED",
        "deterministic_gates_required":gates(),
        "deterministic_gates_run":gates(),
        "output_may_be_used_for_material_signoff":false
    }));
    assert!(contains(&result, "materiality_claim_ceiling_missing"));
}

#[test]
fn materiality_fixture_reader_and_ceiling_shape_fail_closed() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("materiality-fixture-reader");
    let dir = root.join("fixtures/review-materiality/valid");
    std::fs::create_dir_all(&dir).expect("materiality dir");
    std::fs::write(dir.join("bad.json"), "{").expect("bad fixture");
    let failures = crate::review::materiality::fixture_failures(&root);
    assert!(failures.iter().any(|failure| failure.contains("bad.json")));

    let result = errors(json!({
        "decision":"FULL_SCOPE_MATERIAL_REVIEW_REQUIRED",
        "deterministic_gates_required":[],
        "deterministic_gates_run":[],
        "claim_ceiling":{"supported":"release","unsupported":[]},
        "reviewers_required":[],
        "output_may_be_used_for_material_signoff":false,
        "anchors_checked":[],
        "reviewer_registry_evidence":[],
        "material_triggers":[]
    }));
    assert!(contains(&result, "materiality_required_gate_missing"));
    assert!(contains(&result, "materiality_claim_ceiling_missing"));
    std::fs::remove_dir_all(root).expect("cleanup materiality fixture reader");
}

#[test]
fn materiality_review_round_signoff_and_valid_fixture_reader_fail_closed() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "materiality-valid-fixture-reader",
    );
    let dir = root.join("fixtures/review-materiality/valid");
    std::fs::create_dir_all(&dir).expect("materiality dir");
    std::fs::write(
        dir.join("overclaim.json"),
        serde_json::to_vec(&json!({
            "decision":"ADVISORY_REVIEW_ALLOWED",
            "deterministic_gates_required":gates(),
            "deterministic_gates_run":gates(),
            "claim_ceiling":{"supported":["release"],"unsupported":[],"blocked":[]},
            "output_may_be_used_for_material_signoff":true
        }))
        .expect("fixture json"),
    )
    .expect("write materiality fixture");
    let fixture_failures = crate::review::materiality::fixture_failures(&root);
    assert!(
        fixture_failures
            .iter()
            .any(|failure| failure.contains("materiality_advisory_overclaims_support")),
        "{fixture_failures:?}"
    );

    let mut out = Vec::new();
    crate::review::materiality::review_round_errors(
        &json!({
            "round_phase":"sign_off",
            "materiality_gate":{
                "decision":"DELTA_REVIEW_ALLOWED",
                "deterministic_gates_required":gates(),
                "deterministic_gates_run":gates(),
                "claim_ceiling":ceiling(),
                "output_may_be_used_for_material_signoff":false
            }
        }),
        &mut out,
    );
    let got = out
        .iter()
        .map(|failure| failure.error.as_str())
        .collect::<Vec<_>>();
    assert!(
        got.contains(&"material_signoff_requires_full_scope"),
        "{got:?}"
    );
    assert!(got.contains(&"material_signoff_not_authorized"), "{got:?}");
    std::fs::remove_dir_all(root).expect("cleanup materiality valid fixture reader");
}
