use super::*;
use crate::cli::successor::runtime::DiagnosticDetails;

pub(crate) fn execute(
    context: &LiveContext,
    prepared: PreparedFitApply,
    home: &Path,
) -> RuntimeOutcome {
    #[cfg(target_vendor = "apple")]
    {
        return supported::execute(context, prepared, home).unwrap_or_else(host_failure);
    }
    #[cfg(not(target_vendor = "apple"))]
    {
        let _ = (context, prepared, home);
        host_failure(HostFailure::Unsupported)
    }
}

pub(crate) fn recover_pending(
    context: &LiveContext,
    home: &Path,
) -> Result<Option<RuntimeOutcome>, RuntimeOutcome> {
    #[cfg(target_vendor = "apple")]
    {
        return match supported::recover_pending(context, home) {
            Ok(outcome) => Ok(outcome),
            Err(HostFailure::Unavailable) => Ok(None),
            Err(failure) => Err(host_failure(failure)),
        };
    }
    #[cfg(not(target_vendor = "apple"))]
    {
        let _ = (context, home);
        Ok(None)
    }
}

pub(crate) fn host_state_unavailable() -> RuntimeOutcome {
    host_failure(HostFailure::Unavailable)
}

pub(crate) fn production_outcome(outcome: RepositoryFitProductionOutcome) -> RuntimeOutcome {
    let class = match outcome.status() {
        "applied" | "idempotent" | "recovered" => ExitClass::Success,
        "rolled_back" | "interrupted" | "ambiguous" => ExitClass::ActionableFinding,
        _ => match outcome.error_id() {
            Some(AdapterErrorId::ContextStale | AdapterErrorId::StalePlan) => {
                ExitClass::ActionableFinding
            }
            Some(AdapterErrorId::InvalidPlanRecord) => ExitClass::InvalidInvocation,
            Some(
                AdapterErrorId::AcceptanceMismatch
                | AdapterErrorId::PlanConflict
                | AdapterErrorId::ApplyPermitMissing
                | AdapterErrorId::ApplyPermitInvalid
                | AdapterErrorId::ApplyPermitExpired
                | AdapterErrorId::ApplyPermitReplayed
                | AdapterErrorId::ApplyLeaseInvalid,
            ) => ExitClass::BlockedAuthority,
            Some(AdapterErrorId::UnsupportedHost) => ExitClass::UnsupportedCapability,
            Some(
                AdapterErrorId::ApplyMutationScopeViolation
                | AdapterErrorId::ApplyRolledBack
                | AdapterErrorId::ApplyOutcomeAmbiguous,
            ) => ExitClass::ActionableFinding,
            Some(
                AdapterErrorId::InvalidTemplateCatalog
                | AdapterErrorId::TargetUnavailable
                | AdapterErrorId::ProjectionFailed
                | AdapterErrorId::EffectFailed
                | AdapterErrorId::ApplyOutcomeInvalid,
            )
            | None => ExitClass::InternalFailure,
        },
    };
    match outcome.to_machine_bytes() {
        Ok(machine) => RuntimeOutcome::payload(
            class,
            machine,
            format!(
                "repository fit {} result={} effect_started={} rollback_complete={}",
                outcome.status(),
                outcome.result_id(),
                outcome.effect_started(),
                outcome.rollback_complete()
            ),
        ),
        Err(_) => host_failure(HostFailure::Persistence),
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum HostFailure {
    #[cfg(not(target_vendor = "apple"))]
    Unsupported,
    Unavailable,
    Invalid,
    Random,
    Persistence,
    Cleanup,
}

pub(crate) fn host_failure(failure: HostFailure) -> RuntimeOutcome {
    let (class, id, cause, repair, effect, ceiling) = match failure {
        #[cfg(not(target_vendor = "apple"))]
        HostFailure::Unsupported => (
            ExitClass::UnsupportedCapability,
            DiagnosticId::DownstreamToolUnavailable,
            "the public repository-fit writer is supported only on a Darwin host",
            "run fit plan and verify on this host, or use the supported Darwin runtime for apply",
            "none",
            "repository-fit apply remains unavailable on this host",
        ),
        HostFailure::Unavailable => (
            ExitClass::BlockedAuthority,
            DiagnosticId::AuthorityRequired,
            "preprovisioned repository-fit host authority state is unavailable",
            "install or repair the owner-only Harness Ultragoal host state, then rerun the exact accepted plan",
            "none",
            "no workspace effect is authorized or performed",
        ),
        HostFailure::Invalid => (
            ExitClass::BlockedAuthority,
            DiagnosticId::AuthorityRequired,
            "repository-fit host authority state is aliased, substituted, malformed, or unsafe",
            "repair the installed owner-only host state without replacing its authority ledger, then retry",
            "none",
            "no new workspace effect is authorized",
        ),
        HostFailure::Random => (
            ExitClass::BlockedAuthority,
            DiagnosticId::AuthorityRequired,
            "the operating system did not provide a production repository-fit nonce",
            "restore the host random source and recompute the accepted fit request",
            "none",
            "no workspace effect is authorized or performed",
        ),
        HostFailure::Persistence => (
            ExitClass::InternalFailure,
            DiagnosticId::ProjectionFailed,
            "repository-fit authority or recovery state could not be durably reconciled",
            "preserve the owner-only state and diagnose it before any retry",
            "workspace_write_not_acknowledged",
            "repository-fit outcome and dependent claims remain withheld",
        ),
        HostFailure::Cleanup => (
            ExitClass::ActionableFinding,
            DiagnosticId::AuthorityRequired,
            "the workspace outcome returned but its durable recovery envelope could not be safely retired",
            "preserve the target and host state, then rerun fit apply to enter exact recovery",
            "workspace_write_may_have_occurred",
            "repository-fit success and dependent claims remain withheld",
        ),
    };
    RuntimeOutcome::failure(
        class,
        Diagnostic::new(
            id,
            class,
            DiagnosticDetails {
                cause,
                affected_surface: "HCT-FIT public production authority",
                repair,
                effect,
                rerun: "ultragoal --json fit apply --plan <plan> --accept-plan <sha256>",
                ceiling,
            },
        ),
    )
}
