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
        "reviewer_registry_role_exposure",
        "stale_receipt_check",
        "package_private_artifact_hygiene",
        "readiness_validator",
        "claim_ceiling_check",
        "proof_surface_substitution_check",
    ]
}

fn base(decision: &str) -> Value {
    json!({
        "schema":"harness-ultragoal.review-materiality-gate.v2",
        "decision_authority":"deterministic_rust",
        "reviewer_output_authority":"falsification_only_cannot_raise_claims",
        "decision":decision
    })
}

#[test]
fn materiality_value_failures_cover_all_decision_shapes() {
    assert!(contains(
        &errors(json!({"decision":"MAYBE"})),
        "materiality_decision_invalid"
    ));

    let mut missing_value = base("FULL_SCOPE_MATERIAL_REVIEW_REQUIRED");
    missing_value.as_object_mut().unwrap().extend(
        json!({
            "deterministic_gates_required":["anchor_existence_and_digest"],
            "deterministic_gates_run":[],
            "claim_ceiling":ceiling(),
            "reviewers_required":[],
            "output_may_be_used_for_material_signoff":true,
            "anchors_checked":[],
            "reviewer_registry_evidence":[],
            "material_triggers":[]
        })
        .as_object()
        .unwrap()
        .clone(),
    );
    let missing_gate = errors(missing_value);
    assert!(contains(&missing_gate, "materiality_required_gate_missing"));
    assert!(contains(&missing_gate, "materiality_required_gate_not_run"));
    assert!(contains(
        &missing_gate,
        "materiality_full_scope_reviewer_set_missing"
    ));
    assert!(contains(
        &missing_gate,
        "materiality_reviewer_cannot_authorize_signoff"
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

    let mut delta_value = base("DELTA_REVIEW_ALLOWED");
    delta_value.as_object_mut().unwrap().extend(
        json!({
            "deterministic_gates_required":gates(),
            "deterministic_gates_run":gates(),
            "claim_ceiling":ceiling(),
            "reviewers_required":["a","b"],
            "output_may_be_used_for_material_signoff":true
        })
        .as_object()
        .unwrap()
        .clone(),
    );
    let delta = errors(delta_value);
    assert!(contains(&delta, "materiality_delta_used_for_signoff"));
    assert!(contains(
        &delta,
        "materiality_delta_reviewer_scope_too_broad"
    ));

    let mut advisory_value = base("ADVISORY_REVIEW_ALLOWED");
    advisory_value.as_object_mut().unwrap().extend(
        json!({
            "deterministic_gates_required":gates(),
            "deterministic_gates_run":gates(),
            "claim_ceiling":{"supported":["release"],"unsupported":[],"blocked":[]},
            "output_may_be_used_for_material_signoff":true
        })
        .as_object()
        .unwrap()
        .clone(),
    );
    let advisory = errors(advisory_value);
    assert!(contains(&advisory, "materiality_advisory_used_for_signoff"));
    assert!(contains(
        &advisory,
        "materiality_advisory_overclaims_support"
    ));

    let mut blocked_value = base("BLOCKED_BEFORE_REVIEW");
    blocked_value.as_object_mut().unwrap().extend(
        json!({
            "deterministic_gates_required":gates(),
            "deterministic_gates_run":gates(),
            "claim_ceiling":ceiling(),
            "reviewers_required":["claim-falsifier"],
            "validator_repairs_recommended":[]
        })
        .as_object()
        .unwrap()
        .clone(),
    );
    let blocked = errors(blocked_value);
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
    let anchors = crate::self_tests::review::claim_ceiling::anchors();
    crate::review::materiality::review_round_errors(
        &root,
        &json!({
            "review_stage":"falsification",
            "materiality_gate":{
                "schema":"harness-ultragoal.review-materiality-gate.v2",
                "decision_authority":"deterministic_rust",
                "reviewer_output_authority":"falsification_only_cannot_raise_claims",
                "decision":"DELTA_REVIEW_ALLOWED",
                "deterministic_gates_required":gates(),
                "deterministic_gates_run":gates(),
                "claim_ceiling":ceiling(),
                "output_may_be_used_for_material_signoff":true
            }
        }),
        &anchors,
        &mut out,
    );
    let got = out
        .iter()
        .map(|failure| failure.error.as_str())
        .collect::<Vec<_>>();
    assert!(
        got.contains(&"material_falsification_requires_full_scope"),
        "{got:?}"
    );
    assert!(
        got.contains(&"materiality_reviewer_cannot_authorize_signoff"),
        "{got:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup materiality valid fixture reader");
}

#[test]
fn deterministic_materiality_derivation_rejects_forgery_and_substitution() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let fixture = root.join("fixtures/review-round/valid/review-round-receipt.json");
    let receipt = crate::json_boundary::read_json(&fixture).expect("review fixture");
    let anchors = crate::review::round::anchor::values::fixture_anchor_values(&root);
    let check = |value: &Value| {
        let mut out = Vec::new();
        crate::review::materiality::review_round_errors(&root, value, &anchors, &mut out);
        out.into_iter()
            .map(|failure| failure.error)
            .collect::<Vec<_>>()
    };
    let clean = check(&receipt);
    assert!(
        !contains(&clean, "materiality_decision_derivation_mismatch"),
        "{clean:?}"
    );

    let mut forged_delta = receipt.clone();
    forged_delta["materiality_gate"]["decision"] = json!("DELTA_REVIEW_ALLOWED");
    assert!(contains(
        &check(&forged_delta),
        "materiality_decision_derivation_mismatch"
    ));

    let mut missing_trigger = receipt.clone();
    missing_trigger["materiality_gate"]["material_triggers"] = json!([]);
    let got = check(&missing_trigger);
    assert!(contains(&got, "materiality_trigger_missing"));
    assert!(contains(&got, "materiality_decision_derivation_mismatch"));

    let mut tampered_trigger = receipt.clone();
    tampered_trigger["materiality_gate"]["material_triggers"] = json!(["prompt_says_delta"]);
    assert!(contains(
        &check(&tampered_trigger),
        "materiality_trigger_invalid"
    ));

    let mut placeholder = receipt.clone();
    placeholder["materiality_gate"]["anchors_checked"][0]["digest"] =
        json!("sha256:0000000000000000000000000000000000000000000000000000000000000000");
    assert!(contains(
        &check(&placeholder),
        "materiality_anchor_identity_invalid"
    ));

    let mut evidence_swap = receipt.clone();
    evidence_swap["materiality_gate"]["reviewer_registry_evidence"] =
        json!([receipt["materiality_gate"]["anchors_checked"][0].clone()]);
    assert!(contains(
        &check(&evidence_swap),
        "materiality_reviewer_registry_identity_invalid"
    ));

    let mut gate_assertion = receipt;
    gate_assertion["materiality_gate"]["deterministic_gates_run"] = json!([]);
    let got = check(&gate_assertion);
    assert!(contains(&got, "materiality_required_gate_not_run"));
    assert!(contains(&got, "materiality_decision_derivation_mismatch"));
}
