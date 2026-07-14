#[path = "../diagnostics/mod.rs"]
mod diagnostics;
mod dispatch;
mod failures;
mod state_view;

pub(crate) use diagnostics::{Diagnostic, DiagnosticId, RuntimeOutcome};
pub(crate) use dispatch::RuntimeSession;
pub(crate) use state_view::{StateDisposition, StateProjection, StateView};

pub(crate) fn unavailable(invocation: &super::ParsedInvocation) -> RuntimeOutcome {
    let (tool, repair) = failures::downstream(invocation.command);
    failures::delegated(invocation.effect, tool, repair)
}
