use super::*;

pub(crate) fn invalid_invocation() -> RuntimeOutcome {
    RuntimeOutcome::failure(
        ExitClass::InvalidInvocation,
        Diagnostic::new(
            DiagnosticId::UnexpectedArguments,
            ExitClass::InvalidInvocation,
            "the fit adapter received arguments outside the canonical typed route",
            "HCT-FIT public adapter",
            "reparse the exact fit command through the successor grammar",
            "read",
            "ultragoal --json fit inspect",
            "repository-fit and dependent claims remain unchanged",
        ),
    )
}

pub(crate) fn invalid_plan() -> RuntimeOutcome {
    RuntimeOutcome::failure(
        ExitClass::InvalidInvocation,
        Diagnostic::new(
            DiagnosticId::UnexpectedArguments,
            ExitClass::InvalidInvocation,
            "the plan path is not one bounded descriptor-anchored regular file",
            "HCT-FIT accepted plan",
            "write the exact canonical fit plan projection to a confined regular file",
            "read",
            "ultragoal --json fit plan",
            "no workspace effect is authorized or performed",
        ),
    )
}

pub(crate) fn stale_context() -> RuntimeOutcome {
    RuntimeOutcome::failure(
        ExitClass::ActionableFinding,
        Diagnostic::new(
            DiagnosticId::StaleContext,
            ExitClass::ActionableFinding,
            "the target candidate changed during fit plan observation",
            "HCT-FIT target binding",
            "rebuild one target LiveContext and recompute the fit plan",
            "read",
            "ultragoal --json fit plan",
            "no workspace effect is authorized or performed",
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
            failure.cause(),
            "HCT-FIT candidate adapter",
            "rebuild the current target context and exact canonical template-bound fit plan",
            "read_or_unexecuted_workspace_request",
            "ultragoal --json fit plan",
            "no live-repository, readiness, release, or completion claim is raised",
        ),
    )
}
