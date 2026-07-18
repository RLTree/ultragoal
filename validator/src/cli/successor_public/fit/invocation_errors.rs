use super::*;
use crate::cli::successor::runtime::DiagnosticDetails;

pub(crate) fn invalid_invocation() -> RuntimeOutcome {
    RuntimeOutcome::failure(
        ExitClass::InvalidInvocation,
        Diagnostic::new(
            DiagnosticId::UnexpectedArguments,
            ExitClass::InvalidInvocation,
            DiagnosticDetails {
                cause: "the fit adapter received arguments outside the canonical typed route",
                affected_surface: "HCT-FIT public adapter",
                repair: "reparse the exact fit command through the successor grammar",
                effect: "read",
                rerun: "ultragoal --json fit inspect",
                ceiling: "repository-fit and dependent claims remain unchanged",
            },
        ),
    )
}

pub(crate) fn invalid_plan() -> RuntimeOutcome {
    RuntimeOutcome::failure(
        ExitClass::InvalidInvocation,
        Diagnostic::new(
            DiagnosticId::UnexpectedArguments,
            ExitClass::InvalidInvocation,
            DiagnosticDetails {
                cause: "the plan path is not one bounded descriptor-anchored regular file",
                affected_surface: "HCT-FIT accepted plan",
                repair: "write the exact canonical fit plan projection to a confined regular file",
                effect: "read",
                rerun: "ultragoal --json fit plan",
                ceiling: "no workspace effect is authorized or performed",
            },
        ),
    )
}

pub(crate) fn stale_context() -> RuntimeOutcome {
    RuntimeOutcome::failure(
        ExitClass::ActionableFinding,
        Diagnostic::new(
            DiagnosticId::StaleContext,
            ExitClass::ActionableFinding,
            DiagnosticDetails {
                cause: "the target candidate changed during fit plan observation",
                affected_surface: "HCT-FIT target binding",
                repair: "rebuild one target LiveContext and recompute the fit plan",
                effect: "read",
                rerun: "ultragoal --json fit plan",
                ceiling: "no workspace effect is authorized or performed",
            },
        ),
    )
}

pub(crate) fn adapter_failure(failure: FitAdapterError) -> RuntimeOutcome {
    let (class, diagnostic) = match failure.id() {
        AdapterErrorId::ContextStale | AdapterErrorId::StalePlan => {
            (ExitClass::ActionableFinding, DiagnosticId::StaleContext)
        }
        AdapterErrorId::AcceptanceMismatch | AdapterErrorId::PlanConflict => {
            (ExitClass::BlockedAuthority, DiagnosticId::AuthorityRequired)
        }
        AdapterErrorId::InvalidPlanRecord => (
            ExitClass::InvalidInvocation,
            DiagnosticId::UnexpectedArguments,
        ),
        AdapterErrorId::UnsupportedHost => (
            ExitClass::UnsupportedCapability,
            DiagnosticId::DownstreamToolUnavailable,
        ),
        AdapterErrorId::ApplyPermitMissing
        | AdapterErrorId::ApplyPermitInvalid
        | AdapterErrorId::ApplyPermitExpired
        | AdapterErrorId::ApplyPermitReplayed
        | AdapterErrorId::ApplyLeaseInvalid
        | AdapterErrorId::ApplyMutationScopeViolation
        | AdapterErrorId::ApplyOutcomeAmbiguous => {
            (ExitClass::BlockedAuthority, DiagnosticId::AuthorityRequired)
        }
        AdapterErrorId::InvalidTemplateCatalog
        | AdapterErrorId::TargetUnavailable
        | AdapterErrorId::ProjectionFailed
        | AdapterErrorId::EffectFailed
        | AdapterErrorId::ApplyOutcomeInvalid
        | AdapterErrorId::ApplyRolledBack => {
            (ExitClass::InternalFailure, DiagnosticId::ProjectionFailed)
        }
    };
    RuntimeOutcome::failure(
        class,
        Diagnostic::new(
            diagnostic,
            class,
            DiagnosticDetails {
                cause: failure.cause(),
                affected_surface: "HCT-FIT candidate adapter",
                repair: "rebuild the current target context and exact canonical template-bound fit plan",
                effect: "read_or_unexecuted_workspace_request",
                rerun: "ultragoal --json fit plan",
                ceiling: "no live-repository, readiness, release, or completion claim is raised",
            },
        ),
    )
}
