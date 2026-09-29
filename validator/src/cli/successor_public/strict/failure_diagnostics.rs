use crate::cli::successor::runtime::RuntimeOutcome;

pub(super) fn unsupported_claim() -> RuntimeOutcome {
    super::super::failure(
        super::super::DiagnosticId::DownstreamToolUnavailable,
        "the requested claim has no production strict-check adapter",
        "check strict claim routing",
        "select cli-self-law-compliance, namespace-progressive-disclosure, or claim-reconciliation-stage, or implement and register the named claim adapter",
        "the requested strict-check claim remains withheld",
    )
}

pub(super) fn stale_candidate() -> RuntimeOutcome {
    super::super::failure(
        super::super::DiagnosticId::StaleContext,
        "the candidate changed while strict laws were evaluated",
        "check strict candidate binding",
        "stabilize the candidate and rerun the same strict claim",
        "strict-check claims remain withheld",
    )
}

pub(super) fn zero_write_snapshot_unavailable() -> RuntimeOutcome {
    super::super::failure(
        super::super::DiagnosticId::ContextUnavailable,
        "the recursive strict-check zero-write scope could not be captured exactly",
        "check strict zero-write guard",
        "remove unsupported objects or oversized cache material from the active law scope",
        "strict-check claims remain withheld",
    )
}

pub(super) fn hidden_write() -> RuntimeOutcome {
    super::super::failure(
        super::super::DiagnosticId::StaleContext,
        "the strict-check read route changed bytes or metadata in its guarded scope",
        "check strict zero-write guard",
        "repair the hidden writer and rerun against a stable candidate",
        "strict-check claims remain withheld",
    )
}
