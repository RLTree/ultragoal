use super::{RuntimeError, RuntimeErrorId};
use crate::context::{BuildRequest, EffectClass, LiveContext};
use std::path::Path;

use super::super::{InspectTarget, ParsedInvocation, SuccessorCommand};

pub(crate) fn inspect_context(
    start: &Path,
    invocation: &ParsedInvocation,
) -> Result<LiveContext, RuntimeError> {
    if invocation.command != SuccessorCommand::Inspect(InspectTarget::Context) {
        return Err(RuntimeError::new(RuntimeErrorId::WrongCommand));
    }
    if invocation.effect != EffectClass::Read {
        return Err(RuntimeError::new(RuntimeErrorId::EffectMismatch));
    }
    if !invocation.arguments.is_empty() {
        return Err(RuntimeError::new(RuntimeErrorId::UnexpectedArguments));
    }

    let context = LiveContext::build(BuildRequest::new(start).with_effect(EffectClass::Read))
        .map_err(|_| RuntimeError::new(RuntimeErrorId::ContextUnavailable))?;
    context
        .revalidate()
        .map_err(|_| RuntimeError::new(RuntimeErrorId::ContextUnavailable))?;
    Ok(context)
}
