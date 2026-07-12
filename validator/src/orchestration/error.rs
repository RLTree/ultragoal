use std::fmt::{Display, Formatter};

/// Fixed, non-echoing orchestration failures safe for machine and user output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrchestrationError {
    InvalidIdentifier,
    InvalidDigest,
    InvalidPath,
    ResourceLimit,
    DuplicateNode,
    UnknownNode,
    DependencyCycle,
    UnknownScope,
    LeaseConflict,
    RootOnlyScope,
    InvalidLease,
    StaleBinding,
    InvalidWorkerResult,
    ReviewerNotIndependent,
    InvalidReview,
    InvalidEvent,
    ReplayMismatch,
    InvalidTransition,
    RetryExhausted,
    EffectDenied,
    DuplicateOutput,
    JournalIo,
    JournalCorrupt,
    JournalConflict,
    EffectAmbiguous,
    IntegrationAmbiguous,
}

impl OrchestrationError {
    pub fn code(self) -> &'static str {
        match self {
            Self::InvalidIdentifier => "HUL-ORCH-001",
            Self::InvalidDigest => "HUL-ORCH-002",
            Self::InvalidPath => "HUL-ORCH-003",
            Self::ResourceLimit => "HUL-ORCH-004",
            Self::DuplicateNode => "HUL-ORCH-005",
            Self::UnknownNode => "HUL-ORCH-006",
            Self::DependencyCycle => "HUL-ORCH-007",
            Self::UnknownScope => "HUL-ORCH-008",
            Self::LeaseConflict => "HUL-ORCH-009",
            Self::RootOnlyScope => "HUL-ORCH-010",
            Self::InvalidLease => "HUL-ORCH-011",
            Self::StaleBinding => "HUL-ORCH-012",
            Self::InvalidWorkerResult => "HUL-ORCH-013",
            Self::ReviewerNotIndependent => "HUL-ORCH-014",
            Self::InvalidReview => "HUL-ORCH-015",
            Self::InvalidEvent => "HUL-ORCH-016",
            Self::ReplayMismatch => "HUL-ORCH-017",
            Self::InvalidTransition => "HUL-ORCH-018",
            Self::RetryExhausted => "HUL-ORCH-019",
            Self::EffectDenied => "HUL-ORCH-020",
            Self::DuplicateOutput => "HUL-ORCH-021",
            Self::JournalIo => "HUL-ORCH-022",
            Self::JournalCorrupt => "HUL-ORCH-023",
            Self::JournalConflict => "HUL-ORCH-024",
            Self::EffectAmbiguous => "HUL-ORCH-025",
            Self::IntegrationAmbiguous => "HUL-ORCH-026",
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::InvalidIdentifier => "identifier is not canonical",
            Self::InvalidDigest => "digest identity is not canonical",
            Self::InvalidPath => "path is not a confined relative path",
            Self::ResourceLimit => "orchestration input exceeds a bounded limit",
            Self::DuplicateNode => "work graph contains a duplicate node",
            Self::UnknownNode => "work graph references an unknown node",
            Self::DependencyCycle => "work graph contains a dependency cycle",
            Self::UnknownScope => "lease requests undeclared authority",
            Self::LeaseConflict => "active leases overlap",
            Self::RootOnlyScope => "worker requests root-only authority",
            Self::InvalidLease => "lease contract is invalid",
            Self::StaleBinding => "candidate or context binding is stale",
            Self::InvalidWorkerResult => "worker result is malformed or incongruent",
            Self::ReviewerNotIndependent => "reviewer is not independent",
            Self::InvalidReview => "review record is malformed or incongruent",
            Self::InvalidEvent => "event is malformed or tampered",
            Self::ReplayMismatch => "event replay does not match its immutable chain",
            Self::InvalidTransition => "event is not legal from current state",
            Self::RetryExhausted => "bounded retry allowance is exhausted",
            Self::EffectDenied => "effect is outside the active lease",
            Self::DuplicateOutput => "output authority is duplicated",
            Self::JournalIo => "durable journal operation failed",
            Self::JournalCorrupt => "durable journal is missing, malformed, or tampered",
            Self::JournalConflict => "durable journal head changed concurrently",
            Self::EffectAmbiguous => "effect outcome requires root reconciliation",
            Self::IntegrationAmbiguous => "root integration requires observed-state reconciliation",
        }
    }
}

impl Display for OrchestrationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code(), self.message())
    }
}

impl std::error::Error for OrchestrationError {}
