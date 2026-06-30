use super::{prepare_root, write_call_receipt};
use serde_json::json;

#[test]
fn openai_cli_run_paths_and_output_tamper_edges_are_exercised() {
    let root = prepare_root("openai-run-paths");
    let config_receipt = root.join("validation_artifacts/openai/config-run.json");
    let config_args = [
        "openai",
        "config",
        "prove",
        "--receipt",
        config_receipt.to_str().expect("config receipt path"),
    ]
    .into_iter()
    .map(ToString::to_string)
    .collect::<Vec<_>>();
    let config = crate::cli::openai::parse(&config_args)
        .expect("parse config")
        .expect("config command");
    assert_eq!(
        crate::cli::openai::run(&root, &config).expect("run config"),
        0
    );
    assert!(config_receipt.is_file());
    let relative_config = parse_openai(&[
        "openai",
        "config",
        "prove",
        "--receipt",
        "validation_artifacts/openai/config-relative.json",
    ]);
    assert_eq!(
        crate::cli::openai::run(&root, &relative_config).expect("run relative config"),
        0
    );
    assert!(
        root.join("validation_artifacts/openai/config-relative.json")
            .is_file()
    );

    let bad_policy = root.join("docs/openai-bad-key-policy.json");
    crate::self_tests::openai::write_json(
        &bad_policy,
        &json!({
            "schema": "harness-ultragoal.openai-key-policy.v1",
            "law_id": crate::cli::openai::LAW_ID,
            "env_var": "OPENAI_API_KEY",
            "active_destination": ".env",
            "secret_serialization_policy": "forbid",
            "model_output_authority": "observation_only"
        }),
    );
    let bad_config = parse_openai(&[
        "openai",
        "config",
        "prove",
        "--policy",
        bad_policy.to_str().expect("policy path"),
        "--receipt",
        root.join("validation_artifacts/openai/config-bad-run.json")
            .to_str()
            .expect("bad config receipt"),
    ]);
    assert_eq!(
        crate::cli::openai::run(&root, &bad_config).expect("run bad config"),
        1
    );

    let call_receipt = root.join("validation_artifacts/openai/call-run.json");
    let call = parse_openai(&[
        "openai",
        "call",
        "prove",
        "--input-digest",
        &crate::self_tests::boundaries::support::sha('a'),
        "--output-digest",
        &crate::self_tests::boundaries::support::sha('b'),
        "--receipt",
        call_receipt.to_str().expect("call receipt path"),
    ]);
    assert_eq!(crate::cli::openai::run(&root, &call).expect("run call"), 0);
    assert!(call_receipt.is_file());

    let canonical_call_receipt = root.join("validation_artifacts/openai/call-receipt.json");
    std::fs::copy(&call_receipt, &canonical_call_receipt).expect("copy call receipt");
    let output_receipt = root.join("validation_artifacts/openai/output-run.json");
    let output = parse_openai(&[
        "openai",
        "output",
        "prove",
        "--parsed-output-digest",
        &crate::self_tests::boundaries::support::sha('c'),
        "--receipt",
        output_receipt.to_str().expect("output receipt path"),
    ]);
    assert_eq!(
        crate::cli::openai::run(&root, &output).expect("run output"),
        0
    );
    let output_value = crate::json_boundary::read_json(&output_receipt).expect("output json");
    assert!(crate::cli::openai::output::receipt_failures(&root, &output_value).is_empty());

    let bad_output = parse_openai(&[
        "openai",
        "output",
        "prove",
        "--parsed-output-digest",
        "not-a-digest",
        "--parser-schema-id",
        "Authorization: Bearer redacted",
    ]);
    let bad_output_receipt =
        crate::cli::openai::build_receipt(&root, &bad_output).expect("bad output receipt");
    assert_eq!(bad_output_receipt["status"], "fail");
    assert_eq!(bad_output_receipt["redaction_status"], "fail");
    let mut bad_call = crate::json_boundary::read_json(&canonical_call_receipt).expect("call");
    bad_call["status"] = json!("fail");
    bad_call["candidate_digest"] = json!(crate::digest::ZERO);
    crate::json_boundary::write_json(&canonical_call_receipt, &bad_call).expect("bad call");
    let bad_call_output = parse_openai(&[
        "openai",
        "output",
        "prove",
        "--call-receipt",
        canonical_call_receipt.to_str().expect("canonical call"),
        "--parser-schema-id",
        "",
        "--parsed-output-digest",
        &crate::self_tests::boundaries::support::sha('d'),
        "--receipt",
        "validation_artifacts/openai/output-relative.json",
    ]);
    let bad_call_receipt =
        crate::cli::openai::build_receipt(&root, &bad_call_output).expect("bad call output");
    assert!(
        bad_call_receipt["failures"]
            .as_array()
            .expect("failures")
            .iter()
            .any(|item| item == "openai_output_source_call_receipt_not_passing")
    );
    assert!(
        bad_call_receipt["failures"]
            .as_array()
            .expect("failures")
            .iter()
            .any(|item| item == "openai_output_source_call_candidate_mismatch")
    );
    assert!(
        bad_call_receipt["failures"]
            .as_array()
            .expect("failures")
            .iter()
            .any(|item| item == "openai_output_parser_schema_missing")
    );
    assert_eq!(
        crate::cli::openai::run(&root, &bad_call_output).expect("run bad output"),
        1
    );
    assert!(
        root.join("validation_artifacts/openai/output-relative.json")
            .is_file()
    );
    assert_output_receipt_failures();
    assert_openai_parse_failures();
    write_call_receipt(&root);
    std::fs::remove_dir_all(root).expect("cleanup");
}

fn parse_openai(args: &[&str]) -> crate::cli::openai::OpenAiCommand {
    crate::cli::openai::parse(
        &args
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<String>>(),
    )
    .expect("parse openai")
    .expect("openai command")
}

fn assert_output_receipt_failures() {
    let failures =
        crate::cli::openai::output::receipt_failures(std::path::Path::new("."), &json!({}));
    for expected in [
        "openai_model_output_receipt_wrong_schema",
        "openai_model_output_receipt_not_passing",
        "openai_model_output_receipt_candidate_digest_mismatch",
        "openai_model_output_receipt_digest_invalid",
        "openai_model_output_receipt_authority_overbroad",
        "openai_model_output_receipt_secret_leak_or_redaction_failure",
    ] {
        assert!(failures.contains(&expected.to_string()));
    }
}

fn assert_openai_parse_failures() {
    assert!(crate::cli::openai::parse(&["openai".to_string()]).is_err());
    assert!(
        crate::cli::openai::parse(&["not-openai".to_string()])
            .expect("non openai parse")
            .is_none()
    );
    assert!(
        crate::cli::openai::parse(&[
            "openai".to_string(),
            "call".to_string(),
            "prove".to_string(),
            "--mode".to_string(),
            "unknown".to_string(),
        ])
        .is_err()
    );
}
