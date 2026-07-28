use super::*;
use crate::cli::successor::runtime::DiagnosticDetails;
use crate::routine_work::PRODUCTION_SUPPORT_LIMIT;

pub(crate) fn failure(failure: PublicFailure) -> RuntimeOutcome {
    let (class, id, cause, surface, repair, effect, ceiling) = match failure {
        PublicFailure::InvalidInvocation => (
            ExitClass::InvalidInvocation,
            DiagnosticId::UnexpectedArguments,
            "the routine adapter received arguments outside the exact typed check-routine route",
            "routine public invocation",
            "reparse the exact command through the successor grammar",
            "none",
            PRODUCTION_SUPPORT_LIMIT,
        ),
        PublicFailure::Manifest(ManifestFailure::MissingOrUnreadable) => (
            ExitClass::BlockedAuthority,
            DiagnosticId::AuthorityRequired,
            "the adopted routine public manifest is missing or cannot be read safely",
            "routine source authority",
            "restore the exact bounded config/routine-public.json source and retry",
            "none",
            PRODUCTION_SUPPORT_LIMIT,
        ),
        PublicFailure::Manifest(ManifestFailure::Invalid) => (
            ExitClass::BlockedAuthority,
            DiagnosticId::AuthorityRequired,
            "the routine manifest or production catalog is malformed, substituted, or outside the fixed public policy",
            "routine source and command authority",
            "repair the exact adopted manifest and immutable production catalog without broadening runner authority",
            "none",
            PRODUCTION_SUPPORT_LIMIT,
        ),
        PublicFailure::Catalog(code) => (
            ExitClass::BlockedAuthority,
            DiagnosticId::AuthorityRequired,
            code,
            "routine source and command authority",
            "repair the exact adopted manifest and immutable production catalog without broadening runner authority",
            "none",
            PRODUCTION_SUPPORT_LIMIT,
        ),
        PublicFailure::Manifest(ManifestFailure::ConcurrentMutation) => (
            ExitClass::ActionableFinding,
            DiagnosticId::StaleContext,
            "the routine source changed during bounded observation",
            "routine source authority",
            "stabilize the target candidate and rerun from a fresh context",
            "none",
            PRODUCTION_SUPPORT_LIMIT,
        ),
        PublicFailure::Context => (
            ExitClass::ActionableFinding,
            DiagnosticId::ContextUnavailable,
            "an exact workspace-write context could not be constructed for the requested target",
            "routine target binding",
            "stabilize the exact Git worktree and adopted inputs, then retry",
            "none",
            PRODUCTION_SUPPORT_LIMIT,
        ),
        PublicFailure::Routine(error) => routine_failure(error),
        #[cfg(not(target_vendor = "apple"))]
        PublicFailure::Host(HostFailure::Unsupported) => (
            ExitClass::UnsupportedCapability,
            DiagnosticId::DownstreamToolUnavailable,
            "the routine public production mediator is supported only on a Darwin host",
            "routine production host",
            "run the command on the supported Darwin runtime",
            "none",
            PRODUCTION_SUPPORT_LIMIT,
        ),
        PublicFailure::Host(HostFailure::Unavailable) => (
            ExitClass::BlockedAuthority,
            DiagnosticId::AuthorityRequired,
            "owner-only routine host authority is unavailable or unsafe",
            "routine production host authority",
            "install or repair the owner-only routine-public authority, adapter directory, and lock file",
            "none",
            PRODUCTION_SUPPORT_LIMIT,
        ),
        PublicFailure::Host(HostFailure::Busy) => (
            ExitClass::BlockedAuthority,
            DiagnosticId::AuthorityRequired,
            "the exact routine host authority is busy with another active public invocation",
            "routine production host authority",
            "wait for the active invocation to settle, then retry the exact request",
            "none",
            PRODUCTION_SUPPORT_LIMIT,
        ),
        PublicFailure::Host(HostFailure::Invalid) => (
            ExitClass::BlockedAuthority,
            DiagnosticId::AuthorityRequired,
            "routine host authority or reuse state is aliased, stale, forged, malformed, or unsafe",
            "routine production host authority",
            "preserve the ledger, repair the exact owner-only state, and retry only after diagnosis",
            "none",
            PRODUCTION_SUPPORT_LIMIT,
        ),
        PublicFailure::ContinuationUnavailable => (
            ExitClass::BlockedAuthority,
            DiagnosticId::AuthorityRequired,
            "the routine custody already has a terminal or pending record and no exact continuation was supplied",
            "routine continuation authority",
            "retry only with the opaque continuation emitted by a reservation interruption; selectors never grant reuse or recovery authority",
            "none",
            PRODUCTION_SUPPORT_LIMIT,
        ),
        PublicFailure::PersistenceAfterEffect => (
            ExitClass::InternalFailure,
            DiagnosticId::ProjectionFailed,
            "routine execution returned but exact terminal custody did not reconcile",
            "routine production result persistence",
            "preserve the workspace and owner-only ledger, then diagnose before retrying",
            "workspace_write_may_have_occurred",
            PRODUCTION_SUPPORT_LIMIT,
        ),
    };
    RuntimeOutcome::failure(
        class,
        Diagnostic::new(
            id,
            class,
            DiagnosticDetails {
                cause,
                affected_surface: surface,
                repair,
                effect,
                rerun: RERUN,
                ceiling,
            },
        ),
    )
}

pub(crate) fn routine_failure(
    error: RoutineError,
) -> (
    ExitClass,
    DiagnosticId,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
) {
    let (class, id, effect) = match error.id() {
        RoutineErrorId::ConcurrentMutation | RoutineErrorId::ContextMismatch => (
            ExitClass::ActionableFinding,
            DiagnosticId::StaleContext,
            "none_or_unacknowledged_workspace_request",
        ),
        RoutineErrorId::CapabilityUnavailable | RoutineErrorId::UnsupportedEntry => (
            ExitClass::UnsupportedCapability,
            DiagnosticId::DownstreamToolUnavailable,
            "none",
        ),
        RoutineErrorId::InvalidRegistry
        | RoutineErrorId::AmbiguousRegistry
        | RoutineErrorId::UnknownRegistryRow
        | RoutineErrorId::InvalidPath
        | RoutineErrorId::InvalidRequest
        | RoutineErrorId::InvalidSnapshot
        | RoutineErrorId::InvalidReceipt => (
            ExitClass::BlockedAuthority,
            DiagnosticId::AuthorityRequired,
            "none_or_unacknowledged_workspace_request",
        ),
        RoutineErrorId::CaptureFailed
        | RoutineErrorId::CaptureLimit
        | RoutineErrorId::ObservationFailed
        | RoutineErrorId::Serialization => (
            ExitClass::InternalFailure,
            DiagnosticId::ProjectionFailed,
            "none_or_unacknowledged_workspace_request",
        ),
    };
    (
        class,
        id,
        error.cause(),
        "routine production mediation",
        "stabilize the exact target, runner, source, output, and durable local-issuer bindings before retrying",
        effect,
        PRODUCTION_SUPPORT_LIMIT,
    )
}
