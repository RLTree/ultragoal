use super::*;

pub(crate) struct DiagnosisRequest<'a> {
    pub(crate) target: Option<&'a str>,
    pub(crate) finding: Option<&'a str>,
}

pub(crate) fn requests_target(invocation: &ParsedInvocation) -> bool {
    invocation
        .arguments
        .iter()
        .any(|argument| argument.name == OptionName::Target)
}

pub(crate) fn request(invocation: &ParsedInvocation) -> Result<DiagnosisRequest<'_>, ()> {
    if invocation.command != SuccessorCommand::Diagnose || invocation.effect != EffectClass::Read {
        return Err(());
    }
    let mut target = None;
    let mut finding = None;
    for argument in &invocation.arguments {
        match (&argument.name, &argument.value) {
            (OptionName::Target, ParsedValue::RepositoryTarget(value)) if target.is_none() => {
                target = Some(value.as_str());
            }
            (OptionName::Finding, ParsedValue::Identifier(value)) if finding.is_none() => {
                finding = Some(value.as_str());
            }
            _ => return Err(()),
        }
    }
    Ok(DiagnosisRequest { target, finding })
}
