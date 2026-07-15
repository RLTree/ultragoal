use super::*;
use crate::cli::successor::ExitClass;
use crate::routine_work::{
    CHILD_MODE_ENV, CHILD_MODE_VALUE, LEGACY_BEHAVIOR_SELECTOR_ENV, LEGACY_CHILD_SELECTOR_ENV,
    RustSourceSyntaxOutcome, evaluate_rust_source_syntax_frame,
    rust_source_syntax_observation_json,
};
use std::io::Read;

const MAX_FRAME_BYTES: u64 = 16 * 1024 * 1024;

/// Selects only the closed source-syntax parser. This path has no issuer,
/// ledger, cache, or claim access; the parent authenticates its observation.
pub(crate) fn execute_if_requested(invocation: &ParsedInvocation) -> Option<RuntimeOutcome> {
    if legacy_selector_present() {
        return Some(refusal());
    }
    let mode = std::env::var(CHILD_MODE_ENV).ok()?;
    if mode != CHILD_MODE_VALUE
        || invocation.command != SuccessorCommand::Check(CheckProfile::Routine)
        || invocation.effect != EffectClass::WorkspaceWrite
        || !invocation.arguments.is_empty()
    {
        return Some(refusal());
    }
    let mut frame = Vec::new();
    if std::io::stdin()
        .take(MAX_FRAME_BYTES + 1)
        .read_to_end(&mut frame)
        .is_err()
        || frame.len() as u64 > MAX_FRAME_BYTES
    {
        return Some(refusal());
    }
    Some(match evaluate_rust_source_syntax_frame(&frame) {
        RustSourceSyntaxOutcome::Passed(observation) => RuntimeOutcome::payload(
            ExitClass::Success,
            rust_source_syntax_observation_json(&observation),
            "routine behavior observed".to_owned(),
        ),
        RustSourceSyntaxOutcome::Refused(_) => refusal(),
    })
}

fn legacy_selector_present() -> bool {
    std::env::var_os(LEGACY_CHILD_SELECTOR_ENV).is_some()
        || std::env::var_os(LEGACY_BEHAVIOR_SELECTOR_ENV).is_some()
}

fn refusal() -> RuntimeOutcome {
    RuntimeOutcome::payload(
        ExitClass::ActionableFinding,
        br#"{"schema_version":"RoutineBehaviorRefusal-v1","behavior_id":"rust-source-syntax-v1","status":"refused"}"#.to_vec(),
        "routine behavior selector refused".to_owned(),
    )
}
