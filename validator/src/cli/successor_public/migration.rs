use super::*;
use crate::cli::successor::command_contract::{OptionName, ParsedValue};
use crate::inventory::MIGRATION_REGISTRY_PATH;

pub(super) fn plan(context: &LiveContext, invocation: &ParsedInvocation) -> RuntimeOutcome {
    if !valid_registry_argument(invocation) {
        return invalid_registry();
    }
    match InventoryBuilder::new(context).build_migration_plan() {
        Ok(projection) => match projection.to_canonical_json() {
            Ok(machine) if public_output_allowed(machine.len()) => RuntimeOutcome::payload(
                if projection.pending_count() == 0 {
                    ExitClass::Success
                } else {
                    ExitClass::ActionableFinding
                },
                machine,
                format!(
                    "migration plan available items={} pending={} effects={} effects_authorized={}",
                    projection.item_count(),
                    projection.pending_count(),
                    projection.effect_count(),
                    projection.effect_count() > 0,
                ),
            ),
            _ => projection_unavailable(),
        },
        Err(_) => projection_unavailable(),
    }
}

fn invalid_registry() -> RuntimeOutcome {
    RuntimeOutcome::failure(
        ExitClass::InvalidInvocation,
        Diagnostic::new(
            DiagnosticId::UnexpectedArguments,
            ExitClass::InvalidInvocation,
            DiagnosticDetails {
                cause: "migration planning accepts only the canonical adopted registry path",
                affected_surface: "migration plan input",
                repair: "omit --registry or pass the canonical adopted registry path",
                effect: "read",
                rerun: "ultragoal --json migrate plan",
                ceiling: "migration planning and all dependent claims remain unavailable",
            },
        ),
    )
}

fn valid_registry_argument(invocation: &ParsedInvocation) -> bool {
    match invocation.arguments.as_slice() {
        [] => true,
        [argument] if argument.name == OptionName::Registry => {
            matches!(&argument.value, ParsedValue::RelativePath(path) if path.as_str() == MIGRATION_REGISTRY_PATH)
        }
        _ => false,
    }
}

fn projection_unavailable() -> RuntimeOutcome {
    failure(
        DiagnosticId::ProjectionFailed,
        "the current authority inventory and adopted registry did not yield one exact read-only migration plan",
        "ProductMigrationPlanProjection-v2",
        "repair the current semantic inventory or adopted registry without granting migration effects",
        "migration apply, verification, retirement, readiness, and release remain withheld",
    )
}
