use super::*;
use crate::cli::successor::ExitClass;
use crate::routine_work::{
    RustSourceSyntaxOutcome, evaluate_rust_source_syntax_frame, rust_source_syntax_observation_json,
};
use std::io::Read;

const MAX_FRAME_BYTES: u64 = 16 * 1024 * 1024;

pub(crate) fn execute_if_requested(invocation: &ParsedInvocation) -> Option<RuntimeOutcome> {
    let behavior = std::env::var("HUL_ROUTINE_BEHAVIOR_ID").ok()?;
    if behavior != manifest::ROUTINE_BEHAVIOR
        || invocation.command != SuccessorCommand::Check(CheckProfile::Routine)
        || invocation.effect != EffectClass::WorkspaceWrite
        || !invocation.arguments.is_empty()
        || [
            "HUL_ROUTINE_REQUEST_ID",
            "HUL_ROUTINE_PROTOCOL_ID",
            "HUL_ROUTINE_INTENT_ID",
            "HUL_ROUTINE_NODE_ID",
        ]
        .iter()
        .any(|key| std::env::var_os(key).is_none())
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

fn refusal() -> RuntimeOutcome {
    RuntimeOutcome::payload(
        ExitClass::ActionableFinding,
        br#"{"schema_version":"RoutineBehaviorRefusal-v1","behavior_id":"rust-source-syntax-v1","status":"refused"}"#.to_vec(),
        "routine behavior refused".to_owned(),
    )
}
