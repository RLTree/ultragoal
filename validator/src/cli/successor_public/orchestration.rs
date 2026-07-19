use super::*;
use crate::inventory::inspection;
use serde_json::json;

pub(super) fn project(context: &LiveContext, invocation: &ParsedInvocation) -> RuntimeOutcome {
    if invocation.command != SuccessorCommand::Inspect(InspectTarget::Orchestration)
        || invocation.effect != EffectClass::Read
    {
        return RuntimeSession::new(context, None).dispatch(invocation);
    }
    if !invocation.arguments.is_empty() {
        return invalid_invocation();
    }
    if context.revalidate().is_err() {
        return unavailable();
    }
    let projection = match inspection::project(context) {
        Ok(projection) => projection,
        Err(_) => return unavailable(),
    };
    if context.revalidate().is_err() {
        return unavailable();
    }
    let machine = serde_json::to_vec(&json!(projection));
    match machine {
        Ok(machine) if public_output_allowed(machine.len()) => RuntimeOutcome::payload(
            ExitClass::Success,
            machine,
            "canonical orchestration frontier claim_effect=none".to_owned(),
        ),
        _ => unavailable(),
    }
}

fn invalid_invocation() -> RuntimeOutcome {
    RuntimeOutcome::failure(
        ExitClass::InvalidInvocation,
        Diagnostic::new(
            DiagnosticId::UnexpectedArguments,
            ExitClass::InvalidInvocation,
            DiagnosticDetails {
                cause: "orchestration inspection received arguments outside its typed route contract",
                affected_surface: "canonical orchestration inspection",
                repair: "invoke ultragoal --json inspect orchestration without arguments",
                effect: "read",
                rerun: "ultragoal --json inspect orchestration",
                ceiling: "orchestration inspection and dependent claims remain withheld",
            },
        ),
    )
}

fn unavailable() -> RuntimeOutcome {
    RuntimeOutcome::failure(
        ExitClass::UnsupportedCapability,
        Diagnostic::new(
            DiagnosticId::InventoryUnavailable,
            ExitClass::UnsupportedCapability,
            DiagnosticDetails {
                cause: "the current canonical orchestration frontier could not be read safely",
                affected_surface: "canonical orchestration inspection",
                repair: "restore one stable adopted registry and retry the read-only inspection",
                effect: "read",
                rerun: "ultragoal --json inspect orchestration",
                ceiling: "orchestration inspection and dependent claims remain withheld",
            },
        ),
    )
}
