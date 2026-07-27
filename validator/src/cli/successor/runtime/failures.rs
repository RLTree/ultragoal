use super::diagnostics::{Diagnostic, DiagnosticDetails, DiagnosticId, RuntimeOutcome};
use crate::context::EffectClass;

use super::super::{ExitClass, SuccessorCommand};

pub(super) fn downstream(command: SuccessorCommand) -> (&'static str, &'static str) {
    match command {
        SuccessorCommand::Fit(_) => (
            "HCT-FIT",
            "activate the typed repository-fit adapter for this runtime",
        ),
        SuccessorCommand::Check(_) => (
            "HCT-IMPACT",
            "activate the typed routine-work adapter for this runtime",
        ),
        SuccessorCommand::Prove => (
            "HCT-CLAIMS",
            "activate the typed claim-reconciliation adapter for this runtime",
        ),
        SuccessorCommand::Observe(_) => (
            "HCT-OBSERVE",
            "activate the typed observability adapter for this runtime",
        ),
        SuccessorCommand::Package(_) => (
            "HCT-DISTRIBUTION",
            "activate the typed distribution adapter for this runtime",
        ),
        SuccessorCommand::Eval(_) => (
            "HCT-EVAL",
            "activate the authenticated evaluation-admission adapter for this runtime",
        ),
        SuccessorCommand::Migrate(_) => (
            "HCT-MIGRATE",
            "activate the typed migration adapter for this runtime",
        ),
        _ => (
            "HCT-STATE",
            "activate the typed product-state adapter for this runtime",
        ),
    }
}

pub(super) fn delegated(
    effect: EffectClass,
    tool: &'static str,
    repair: &'static str,
) -> RuntimeOutcome {
    let authority = matches!(
        effect,
        EffectClass::ExternalWrite | EffectClass::Destructive
    );
    let class = if authority {
        ExitClass::BlockedAuthority
    } else {
        ExitClass::UnsupportedCapability
    };
    RuntimeOutcome::failure(
        class,
        Diagnostic::new(
            if authority {
                DiagnosticId::AuthorityRequired
            } else {
                DiagnosticId::DownstreamToolUnavailable
            },
            class,
            DiagnosticDetails {
                cause: if authority {
                    "the command requires authority and a product adapter not active in this runtime"
                } else {
                    "the command belongs to a downstream Harness tool that is not wired into this runtime"
                },
                affected_surface: tool,
                repair,
                effect: if authority {
                    "external-or-destructive"
                } else {
                    "delegated"
                },
                rerun: "ultragoal --json inspect findings",
                ceiling: "no downstream product or readiness claim is raised",
            },
        ),
    )
}

pub(super) fn effect_mismatch() -> RuntimeOutcome {
    failure(
        DiagnosticId::EffectMismatch,
        ExitClass::InternalFailure,
        "parsed command effect does not match the canonical command catalog or live context",
        "successor runtime",
        "reparse through the canonical typed parser and rebuild one matching LiveContext",
        "ultragoal --json inspect context",
        "runtime claim withheld",
    )
}

pub(super) fn unexpected_arguments() -> RuntimeOutcome {
    failure(
        DiagnosticId::UnexpectedArguments,
        ExitClass::InvalidInvocation,
        "runtime received arguments outside the typed route contract",
        "successor runtime",
        "invoke the route through the canonical parser without injected arguments",
        "ultragoal --help",
        "runtime claim withheld",
    )
}

pub(super) fn stale_context() -> RuntimeOutcome {
    failure(
        DiagnosticId::StaleContext,
        ExitClass::ActionableFinding,
        "the live candidate changed during the read operation",
        "candidate-bound runtime",
        "rebuild one LiveContext and recompute dependent state from that candidate",
        "ultragoal --json inspect context",
        "same-candidate claims invalidated",
    )
}

pub(super) fn state_unavailable() -> RuntimeOutcome {
    failure(
        DiagnosticId::StateUnavailable,
        ExitClass::UnsupportedCapability,
        "no context-matched HCT-STATE graph was supplied to the runtime session",
        "inspect next diagnose",
        "derive ProductState from the same LiveContext and HCT-INVENTORY catalog",
        "ultragoal --json inspect context",
        "state and completion claims remain withheld",
    )
}

pub(super) fn state_context_mismatch() -> RuntimeOutcome {
    failure(
        DiagnosticId::StateContextMismatch,
        ExitClass::InternalFailure,
        "the supplied HCT-STATE graph was derived from a different LiveContext",
        "inspect next diagnose",
        "discard the projection and derive ProductState from the current session context",
        "ultragoal --json inspect context",
        "same-candidate state and completion claims withheld",
    )
}

pub(super) fn projection_failure() -> RuntimeOutcome {
    failure(
        DiagnosticId::ProjectionFailed,
        ExitClass::InternalFailure,
        "a bounded versioned runtime projection could not be produced",
        "successor runtime output",
        "recompute the typed graph and repair the projection at its source",
        "ultragoal --json inspect context",
        "runtime claim withheld",
    )
}

fn failure(
    id: DiagnosticId,
    class: ExitClass,
    cause: &'static str,
    surface: &'static str,
    repair: &'static str,
    rerun: &'static str,
    ceiling: &'static str,
) -> RuntimeOutcome {
    RuntimeOutcome::failure(
        class,
        Diagnostic::new(
            id,
            class,
            DiagnosticDetails {
                cause,
                affected_surface: surface,
                repair,
                effect: "read",
                rerun,
                ceiling,
            },
        ),
    )
}
