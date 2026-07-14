use super::*;
use crate::cli::successor::ExitClass;
use crate::routine_work::BROKER_CHILD_REQUEST_ENV;

/// Detects an attempted broker-child invocation, but cannot authorize it.
///
/// The root-owned broker contract is deliberately absent from this source
/// package.  Until root wiring supplies an opaque, non-serializable grant at a
/// stronger trust boundary, every child request refuses before inspecting any
/// descriptor or framed input.
pub(crate) fn execute_if_requested(_invocation: &ParsedInvocation) -> Option<RuntimeOutcome> {
    std::env::var_os(BROKER_CHILD_REQUEST_ENV).map(|_| refusal())
}

fn refusal() -> RuntimeOutcome {
    RuntimeOutcome::payload(
        ExitClass::ActionableFinding,
        br#"{"schema_version":"RoutineBehaviorRefusal-v1","behavior_id":"rust-source-syntax-v1","status":"refused"}"#.to_vec(),
        "routine behavior requires root broker authorization".to_owned(),
    )
}
