use super::*;

pub(super) fn project(context: &LiveContext, invocation: &ParsedInvocation) -> RuntimeOutcome {
    if invocation.command != SuccessorCommand::Inspect(InspectTarget::Inception)
        || invocation.effect != EffectClass::Read
    {
        return RuntimeSession::new(context, None).dispatch(invocation);
    }
    if !invocation.arguments.is_empty() {
        return failure(
            ExitClass::InvalidInvocation,
            DiagnosticId::UnexpectedArguments,
        );
    }
    match crate::product_inception::inspect(context) {
        Ok(projection) => match projection.to_json() {
            Ok(machine) if public_output_allowed(machine.len()) => RuntimeOutcome::payload(
                ExitClass::Success,
                machine,
                "Current product authority inception projection is current".to_owned(),
            ),
            _ => failure(ExitClass::InternalFailure, DiagnosticId::ProjectionFailed),
        },
        Err(error) => failure_with_cause(
            ExitClass::ActionableFinding,
            DiagnosticId::InventoryUnavailable,
            error.cause(),
        ),
    }
}

fn failure(class: ExitClass, id: DiagnosticId) -> RuntimeOutcome {
    failure_with_cause(
        class,
        id,
        "the current product authority inputs could not be projected safely",
    )
}

fn failure_with_cause(class: ExitClass, id: DiagnosticId, cause: &'static str) -> RuntimeOutcome {
    RuntimeOutcome::failure(
        class,
        Diagnostic::new(
            id,
            class,
            DiagnosticDetails {
                cause,
                affected_surface: "read-only current product authority inception",
                repair: "restore exactly one safe current goal, Product Success Contract, and active ExecPlan owner, then retry",
                effect: "read",
                rerun: "ultragoal --json inspect inception",
                ceiling: "current product authority inspection is withheld; CL-USABLE-LOOP remains withheld pending same-surface evaluator evidence",
            },
        ),
    )
}
