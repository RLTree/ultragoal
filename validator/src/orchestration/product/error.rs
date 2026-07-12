use crate::orchestration::OrchestrationError;
use std::fmt::{Display, Formatter};

/// Stable, non-echoing failures for the product orchestration boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductError {
    AuthorityInvalid,
    AuthorityExpired,
    AuthorityOperationMismatch,
    StaleCandidate,
    ConcurrentUpdate,
    UnknownWorker,
    UnknownArtifact,
    UnknownOperation,
    ResultSubstitution,
    AmbiguousRecovery,
    LeaseExpired,
    WorkspaceChanged,
    InvalidWorkspacePath,
    Kernel(OrchestrationError),
}

impl ProductError {
    pub fn code(self) -> &'static str {
        match self {
            Self::AuthorityInvalid => "HUL-ORCH-PROD-001",
            Self::AuthorityExpired => "HUL-ORCH-PROD-002",
            Self::AuthorityOperationMismatch => "HUL-ORCH-PROD-003",
            Self::StaleCandidate => "HUL-ORCH-PROD-004",
            Self::ConcurrentUpdate => "HUL-ORCH-PROD-005",
            Self::UnknownWorker => "HUL-ORCH-PROD-006",
            Self::UnknownArtifact => "HUL-ORCH-PROD-007",
            Self::UnknownOperation => "HUL-ORCH-PROD-008",
            Self::ResultSubstitution => "HUL-ORCH-PROD-009",
            Self::AmbiguousRecovery => "HUL-ORCH-PROD-010",
            Self::LeaseExpired => "HUL-ORCH-PROD-011",
            Self::WorkspaceChanged => "HUL-ORCH-PROD-012",
            Self::InvalidWorkspacePath => "HUL-ORCH-PROD-013",
            Self::Kernel(error) => error.code(),
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::AuthorityInvalid => "root authority is absent or invalid",
            Self::AuthorityExpired => "root authority has expired",
            Self::AuthorityOperationMismatch => "root authority does not match the operation",
            Self::StaleCandidate => "candidate or context binding is stale",
            Self::ConcurrentUpdate => "another reconciler changed the authoritative journal",
            Self::UnknownWorker => "worker or lease is not part of the current journal",
            Self::UnknownArtifact => "artifact is outside the bound lease authority",
            Self::UnknownOperation => "operation is not pending in the current journal",
            Self::ResultSubstitution => "result commitment does not match the bound result",
            Self::AmbiguousRecovery => "recovery requires an explicit reconciliation first",
            Self::LeaseExpired => "lease cannot resume after its deadline",
            Self::WorkspaceChanged => "the anchored orchestration workspace changed",
            Self::InvalidWorkspacePath => "orchestration workspace path is not confined",
            Self::Kernel(error) => return error_message(error),
        }
    }
}

fn error_message(error: OrchestrationError) -> &'static str {
    match error {
        OrchestrationError::InvalidIdentifier => "identifier is not canonical",
        OrchestrationError::InvalidDigest => "digest identity is not canonical",
        OrchestrationError::InvalidPath => "path is not a confined relative path",
        OrchestrationError::ResourceLimit => "orchestration input exceeds a bounded limit",
        OrchestrationError::DuplicateNode => "work graph contains a duplicate node",
        OrchestrationError::UnknownNode => "work graph references an unknown node",
        OrchestrationError::DependencyCycle => "work graph contains a dependency cycle",
        OrchestrationError::UnknownScope => "lease requests undeclared authority",
        OrchestrationError::LeaseConflict => "active leases overlap",
        OrchestrationError::RootOnlyScope => "worker requests root-only authority",
        OrchestrationError::InvalidLease => "lease contract is invalid",
        OrchestrationError::StaleBinding => "candidate or context binding is stale",
        OrchestrationError::InvalidWorkerResult => "worker result is malformed or incongruent",
        OrchestrationError::ReviewerNotIndependent => "reviewer is not independent",
        OrchestrationError::InvalidReview => "review record is malformed or incongruent",
        OrchestrationError::InvalidEvent => "event is malformed or tampered",
        OrchestrationError::ReplayMismatch => "event replay does not match its immutable chain",
        OrchestrationError::InvalidTransition => "event is not legal from current state",
        OrchestrationError::RetryExhausted => "bounded retry allowance is exhausted",
        OrchestrationError::EffectDenied => "effect is outside the active lease",
        OrchestrationError::DuplicateOutput => "output authority is duplicated",
        OrchestrationError::JournalIo => "durable journal operation failed",
        OrchestrationError::JournalCorrupt => "durable journal is missing, malformed, or tampered",
        OrchestrationError::JournalConflict => "durable journal head changed concurrently",
        OrchestrationError::EffectAmbiguous => "effect outcome requires root reconciliation",
        OrchestrationError::IntegrationAmbiguous => {
            "root integration requires observed-state reconciliation"
        }
    }
}

impl From<OrchestrationError> for ProductError {
    fn from(error: OrchestrationError) -> Self {
        match error {
            OrchestrationError::StaleBinding => Self::StaleCandidate,
            OrchestrationError::JournalConflict => Self::ConcurrentUpdate,
            OrchestrationError::EffectAmbiguous | OrchestrationError::IntegrationAmbiguous => {
                Self::AmbiguousRecovery
            }
            other => Self::Kernel(other),
        }
    }
}

impl Display for ProductError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code(), self.message())
    }
}

impl std::error::Error for ProductError {}
