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
            Ok(machine) if public_output_allowed(machine.len()) => {
                let missing =
                    matches!(projection, crate::product_inception::Projection::Missing(_));
                RuntimeOutcome::payload(
                    if missing {
                        ExitClass::ActionableFinding
                    } else {
                        ExitClass::Success
                    },
                    machine,
                    if missing {
                        "Product Success Brief inputs are required before evidence-led ranking"
                    } else {
                        "Product Success Brief inception projection is current"
                    }
                    .to_owned(),
                )
            }
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
        "the Product Success Brief inception input could not be projected safely",
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
                affected_surface: "read-only Product Success Brief inception",
                repair: "repair the canonical brief or its current authority bindings and retry",
                effect: "read",
                rerun: "ultragoal --json inspect inception",
                ceiling: "evidence-led ranking and dependent product claims remain withheld",
            },
        ),
    )
}
