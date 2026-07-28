use super::*;
use crate::cli::successor::{EffectClass, OptionArgument, OutputMode};
use crate::cli::successor_public::repository_fixture::Repository;
use std::fs;

fn invocation(value: ParsedValue) -> ParsedInvocation {
    ParsedInvocation {
        command: SuccessorCommand::Check(CheckProfile::Strict),
        effect: EffectClass::Read,
        arguments: vec![OptionArgument {
            name: OptionName::Claim,
            value,
        }],
        output_mode: OutputMode::Json,
    }
}

#[test]
fn strict_claim_boundary_accepts_only_identifier_value() {
    let valid = invocation(ParsedValue::Identifier(SELF_LAW_CLAIM.to_owned()));
    assert_eq!(claim_id(&valid), Some(SELF_LAW_CLAIM));

    let namespace = invocation(ParsedValue::Identifier(NAMESPACE_LAW_CLAIM.to_owned()));
    assert_eq!(claim_id(&namespace), Some(NAMESPACE_LAW_CLAIM));

    let staged = invocation(ParsedValue::Identifier(
        CLAIM_RECONCILIATION_STAGE.to_owned(),
    ));
    assert_eq!(claim_id(&staged), Some(CLAIM_RECONCILIATION_STAGE));

    let invalid = invocation(ParsedValue::Flag);
    assert_eq!(claim_id(&invalid), None);
}

#[test]
fn strict_claim_boundary_rejects_wrong_command() {
    let mut value = invocation(ParsedValue::Identifier(SELF_LAW_CLAIM.to_owned()));
    value.command = SuccessorCommand::Check(CheckProfile::Routine);
    assert_eq!(claim_id(&value), None);
}

#[test]
fn public_self_law_refuses_repository_python_and_fails_the_subcheck_honestly() {
    let repo = Repository::new("strict-repository-python-refusal");
    let outside = repo.root.with_file_name(format!(
        "{}-outside-sentinel",
        repo.root.file_name().unwrap().to_string_lossy()
    ));
    fs::create_dir_all(repo.root.join("scripts")).unwrap();
    let outside_relative = format!("../{}", outside.file_name().unwrap().to_string_lossy());
    let outside_literal = serde_json::to_string(&outside_relative).unwrap();
    fs::write(
        repo.root.join("scripts/check-python-source-laws"),
        format!(
            "from pathlib import Path\nPath({outside_literal}).write_bytes(b'escaped')\nprint('python source laws pass: 1 typed sources')\n"
        ),
    )
    .unwrap();

    let streams = execute(
        &repo.root,
        &invocation(ParsedValue::Identifier(SELF_LAW_CLAIM.to_owned())),
    )
    .render(OutputMode::Json);

    assert_eq!(streams.exit_code, 1);
    assert!(!outside.exists());
    assert!(streams.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&streams.stdout).unwrap();
    assert_eq!(value["status"], "fail");
    assert_eq!(value["supported_claims"], serde_json::json!([]));
    assert!(value["findings"].as_array().is_some_and(|findings| {
        findings.iter().any(|finding| {
            finding["check_id"] == "python-source-laws"
                && finding["detail"]
                    == "adapter_error:trusted_compiled_python_source_law_validation_unavailable"
        })
    }));
}
