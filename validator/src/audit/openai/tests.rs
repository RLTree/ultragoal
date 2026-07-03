use serde_json::json;

fn root(label: &str) -> std::path::PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    std::fs::create_dir_all(root.join("validation_artifacts/openai")).expect("openai dir");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    )
    .expect("manifest");
    root
}

#[test]
fn openai_receipt_audit_rejects_missing_wrong_schema_and_secret_shapes() {
    let root = root("audit-openai-receipts");
    let mut out = Vec::new();
    super::check_receipt(&root, &mut out);
    assert!(
        out.iter()
            .any(|failure| failure.starts_with("openai_config_receipt_missing_or_malformed")),
        "{out:#?}"
    );

    crate::json_boundary::write_json(
        &root.join(super::RECEIPT_REL),
        &json!({
            "schema":"wrong",
            "status":"pass",
            "candidate_digest":"sha256:bad",
            "redaction_status":"pass",
            "secret_material_serialized":false,
            "claim_ceiling":"openai_config_resolution_only",
            "blocked_claims":["completion","readiness","release"],
            "observability_receipt":{"schema":"wrong"}
        }),
    )
    .expect("config receipt");
    let mut wrong = Vec::new();
    super::check_receipt(&root, &mut wrong);
    assert!(wrong.contains(&"openai_config_receipt_wrong_schema".to_string()));
    assert!(wrong.contains(&"openai_config_receipt_candidate_digest_mismatch".to_string()));
    assert!(wrong.contains(&"openai_config_receipt_missing_completion_blockers".to_string()));

    let mut failing_config =
        crate::json_boundary::read_json(&root.join(super::RECEIPT_REL)).expect("config");
    failing_config["status"] = json!("fail");
    failing_config["redaction_status"] = json!("fail");
    failing_config["secret_material_serialized"] = json!(true);
    failing_config["claim_ceiling"] = json!("completion");
    crate::json_boundary::write_json(&root.join(super::RECEIPT_REL), &failing_config)
        .expect("failing config");
    let mut failing = Vec::new();
    super::check_receipt(&root, &mut failing);
    assert!(failing.contains(&"openai_config_receipt_not_passing".to_string()));
    assert!(
        failing.contains(&"openai_config_receipt_secret_leak_or_redaction_failure".to_string())
    );
    assert!(failing.contains(&"openai_config_receipt_secret_material_serialized".to_string()));
    assert!(failing.contains(&"openai_config_receipt_claim_ceiling_overbroad".to_string()));

    let mut call = Vec::new();
    super::check_call_receipt(&root, &mut call);
    assert!(
        call.iter()
            .any(|failure| failure.starts_with("openai_call_receipt_missing_or_malformed")),
        "{call:#?}"
    );
    crate::json_boundary::write_json(
        &root.join(super::CALL_RECEIPT_REL),
        &json!({
            "schema":"wrong",
            "status":"fail",
            "candidate_digest":"sha256:bad",
            "redaction_status":"fail",
            "prompt_input_digest":"not-a-digest",
            "output_digest":"sha256:bad",
            "blocked_claims":["completion"],
            "model_output_authority":"too_broad"
        }),
    )
    .expect("call receipt");
    let mut call_wrong = Vec::new();
    super::check_call_receipt(&root, &mut call_wrong);
    assert!(call_wrong.contains(&"openai_call_receipt_wrong_schema".to_string()));
    assert!(call_wrong.contains(&"openai_call_receipt_not_passing".to_string()));
    assert!(call_wrong.contains(&"openai_call_receipt_digest_fields_invalid".to_string()));
}

#[test]
fn openai_observability_binding_and_digest_guards_are_strict() {
    let mut out = Vec::new();
    super::check_observability_binding(&json!({}), "sha256:a", "openai_test", &mut out);
    assert_eq!(
        out,
        vec!["openai_test_receipt_missing_observability_binding".to_string()]
    );

    let mut invalid = Vec::new();
    super::check_observability_binding(
        &json!({"observability_receipt":{"schema":"wrong","status":"pass"}}),
        "sha256:a",
        "openai_test",
        &mut invalid,
    );
    assert_eq!(
        invalid,
        vec!["openai_test_receipt_observability_binding_invalid".to_string()]
    );

    let complete = json!({"blocked_claims":[
        "completion",
        "readiness",
        "release",
        "final_packet_correctness",
        "update_goal_eligibility",
        "model_output_authority"
    ]});
    assert!(super::blocks_completion_claims(&complete));
    assert!(!super::blocks_completion_claims(
        &json!({"blocked_claims":["completion"]})
    ));
    assert!(super::valid_digest(&json!({"d": crate::digest::ZERO}), "d"));
    assert!(!super::valid_digest(&json!({"d":"sha256:bad"}), "d"));
}
