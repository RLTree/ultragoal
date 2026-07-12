use serde_json::json;

fn errors(value: &serde_json::Value) -> Vec<String> {
    let mut out = Vec::new();
    crate::review::round::runtime::receipt_errors(value, &mut out);
    out.into_iter().map(|failure| failure.error).collect()
}

#[test]
fn unavailable_runtime_metadata_rejects_hidden_model_inference_and_overclaim() {
    let got = errors(&json!({
        "review_authority":"falsification_evidence_only",
        "runtime_metadata_ceiling":"unavailable_no_runtime_configuration_claim",
        "claim_ceiling":{
            "supported":[{"claim_id":"reviewer_runtime_configuration"}],
            "unsupported":[]
        },
        "reviewers":[{
            "role":"claim-falsifier",
            "runtime_metadata":{"exposure":"unavailable","model":"inferred-model"}
        }]
    }));
    assert!(
        got.contains(&"review_round_runtime_metadata_unexposed_values".to_string()),
        "{got:?}"
    );
    assert!(
        got.contains(&"review_round_runtime_metadata_overclaim".to_string()),
        "{got:?}"
    );
}

#[test]
fn host_exposed_runtime_metadata_is_unconditionally_withheld() {
    let got = errors(&json!({
        "review_authority":"falsification_evidence_only",
        "runtime_metadata_ceiling":"host_exposed_values_bound",
        "claim_ceiling":{"supported":[],"unsupported":[]},
        "reviewers":[{
            "role":"security-reviewer",
            "runtime_metadata":{"exposure":"host_exposed","model":"gpt-host","mode":"read-only"}
        }]
    }));
    assert!(
        got.contains(&"review_round_runtime_metadata_host_exposed_unbound".to_string()),
        "{got:?}"
    );
}

#[test]
fn matching_spawn_and_prompt_runtime_values_cannot_forge_host_provenance() {
    let metadata = json!({
        "exposure":"host_exposed",
        "model":"prompt-model",
        "mode":"prompt-mode",
        "reasoning_effort":"prompt-reasoning"
    });
    assert!(!crate::review::round::runtime::metadata_matches(
        &json!({"runtime_metadata":metadata.clone()}),
        &json!({"runtime_metadata":metadata.clone()})
    ));
    let got = errors(&json!({
        "review_authority":"falsification_evidence_only",
        "runtime_metadata_ceiling":"host_exposed_values_bound",
        "claim_ceiling":{
            "supported":[],
            "unsupported":[
                {"claim_id":"reviewer_runtime_configuration"},
                {"claim_id":"custom_agent_runtime_discovery"}
            ]
        },
        "reviewers":[{"role":"claim-falsifier","runtime_metadata":metadata}]
    }));
    for expected in [
        "review_round_runtime_metadata_host_exposed_unbound",
        "review_round_runtime_metadata_ceiling_mismatch",
    ] {
        assert!(got.contains(&expected.to_string()), "{expected}: {got:?}");
    }
}

#[test]
fn mixed_runtime_exposure_lowers_the_receipt_ceiling() {
    let got = errors(&json!({
        "review_authority":"approval_authority",
        "runtime_metadata_ceiling":"host_exposed_values_bound",
        "claim_ceiling":{
            "supported":[],
            "unsupported":[{"claim_id":"reviewer_runtime_configuration"}]
        },
        "reviewers":[
            {
                "role":"claim-falsifier",
                "runtime_metadata":{"exposure":"host_exposed","model":"host-model","mode":"read-only","reasoning_effort":"high"}
            },
            {
                "role":"product-journey-reviewer",
                "runtime_metadata":{"exposure":"unavailable"}
            }
        ]
    }));
    assert!(
        got.contains(&"review_round_review_authority_invalid".to_string()),
        "{got:?}"
    );
    assert!(
        got.contains(&"review_round_runtime_metadata_ceiling_mismatch".to_string()),
        "{got:?}"
    );
}

#[test]
fn runtime_failures_never_echo_attacker_controlled_role_or_metadata() {
    let mut failures = Vec::new();
    crate::review::round::runtime::receipt_errors(
        &json!({
            "review_authority":"falsification_evidence_only",
            "runtime_metadata_ceiling":"unavailable_no_runtime_configuration_claim",
            "claim_ceiling":{
                "supported":[],
                "unsupported":[
                    {"claim_id":"reviewer_runtime_configuration"},
                    {"claim_id":"custom_agent_runtime_discovery"}
                ]
            },
            "reviewers":[{
                "role":"SECRET_CANARY",
                "runtime_metadata":{"exposure":"host_exposed","model":"SECRET_CANARY"}
            }]
        }),
        &mut failures,
    );
    let rendered = failures
        .iter()
        .map(|failure| format!("{}:{}", failure.error, failure.detail))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!rendered.contains("SECRET_CANARY"), "{rendered}");
}

#[test]
fn full_persona_validation_never_echoes_prior_duplicate_id_or_unknown_role() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let mut receipt = crate::json_boundary::read_json(
        &root.join("fixtures/review-round/valid/review-round-receipt.json"),
    )
    .expect("review receipt");
    receipt["prior_round_reviewer_agent_ids"]
        .as_array_mut()
        .expect("prior ids")
        .push(json!("SECRET_CANARY"));
    receipt["reviewers"][0]["reviewer_agent_id"] = json!("SECRET_CANARY");
    receipt["reviewers"][1]["reviewer_agent_id"] = json!("SECRET_CANARY");
    receipt["reviewers"][2]["role"] = json!("SECRET_CANARY");
    receipt["reviewers"][3]["role"] = json!("SECRET_CANARY");

    let scratch = crate::self_tests::boundaries::workspace_fixtures::temp_root("persona-non-echo");
    std::fs::create_dir_all(&scratch).expect("scratch");
    let receipt_path = scratch.join("receipt.json");
    std::fs::write(&receipt_path, serde_json::to_vec(&receipt).unwrap()).expect("receipt");
    let anchors = crate::review::round::AnchorPaths {
        validator_receipt: root.join("validation_artifacts/ultragoal-audit/validator-receipt.json"),
        review_target_receipt: root.join("validation_artifacts/review/review-target-receipt.json"),
        archive_receipt: root.join("validation_artifacts/review/candidate-archive-receipt.json"),
    };
    let error = crate::review::round::validate_files(&root, &receipt_path, &anchors)
        .expect_err("attacker-controlled persona fields rejected");
    assert!(error.contains("review_round_reused_reviewer"), "{error}");
    assert!(error.contains("review_round_duplicate_role"), "{error}");
    assert!(!error.contains("SECRET_CANARY"), "{error}");
    std::fs::remove_dir_all(scratch).expect("cleanup persona canary");
}
