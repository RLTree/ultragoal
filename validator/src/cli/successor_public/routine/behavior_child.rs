use super::*;
use crate::cli::successor::ExitClass;
use crate::routine_work::{
    CHILD_MODE_ENV, CHILD_MODE_VALUE, LEGACY_BEHAVIOR_SELECTOR_ENV, LEGACY_CHILD_SELECTOR_ENV,
    RustSourceSyntaxOutcome, activate_and_read_frame, evaluate_rust_source_syntax_frame,
    refusal_json, rust_source_syntax_observation_json,
};

const MAX_FRAME_BYTES: u64 = 16 * 1024 * 1024;

/// Selects only the closed source-syntax parser. This path has no issuer,
/// ledger, cache, or claim access; the parent authenticates its observation.
pub(crate) fn execute_if_requested(invocation: &ParsedInvocation) -> Option<RuntimeOutcome> {
    if legacy_selector_present() {
        return Some(refusal("legacy_selector_present"));
    }
    let mode = std::env::var(CHILD_MODE_ENV).ok()?;
    if mode != CHILD_MODE_VALUE
        || invocation.command != SuccessorCommand::Check(CheckProfile::Routine)
        || invocation.effect != EffectClass::WorkspaceWrite
        || !invocation.arguments.is_empty()
    {
        return Some(refusal("invocation_not_authorized"));
    }
    let frame = match activate_and_read_frame(std::io::stdin(), MAX_FRAME_BYTES) {
        Ok(frame) => frame,
        Err(error) => return Some(refusal(error.code())),
    };
    Some(match evaluate_rust_source_syntax_frame(&frame) {
        RustSourceSyntaxOutcome::Passed(observation) => RuntimeOutcome::payload(
            ExitClass::Success,
            rust_source_syntax_observation_json(&observation),
            "routine behavior observed".to_owned(),
        ),
        RustSourceSyntaxOutcome::Refused(_) => refusal("rust_source_syntax_refused"),
    })
}

fn legacy_selector_present() -> bool {
    std::env::var_os(LEGACY_CHILD_SELECTOR_ENV).is_some()
        || std::env::var_os(LEGACY_BEHAVIOR_SELECTOR_ENV).is_some()
}

fn refusal(reason: &'static str) -> RuntimeOutcome {
    RuntimeOutcome::payload(
        ExitClass::ActionableFinding,
        refusal_json(reason),
        "routine behavior selector refused".to_owned(),
    )
}
