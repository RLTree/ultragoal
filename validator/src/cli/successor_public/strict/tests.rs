use super::*;
use crate::cli::successor::{EffectClass, OptionArgument, OutputMode};

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
