use sha2::{Digest, Sha256};
use std::fmt;

use super::runtime_adapter::{LaunchCleanupEvidence, ObservedProcessCustody};

#[path = "error/failure_evidence.rs"]
mod failure_evidence;

pub(crate) use failure_evidence::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RoutineErrorId {
    InvalidRegistry,
    AmbiguousRegistry,
    UnknownRegistryRow,
    InvalidPath,
    UnsupportedEntry,
    ConcurrentMutation,
    ContextMismatch,
    CapabilityUnavailable,
    CaptureFailed,
    CaptureLimit,
    InvalidSnapshot,
    InvalidRequest,
    InvalidReceipt,
    ObservationFailed,
    Serialization,
}

impl RoutineErrorId {
    pub fn code(self) -> &'static str {
        match self {
            Self::InvalidRegistry => "RWT-REGISTRY-INVALID",
            Self::AmbiguousRegistry => "RWT-REGISTRY-AMBIGUOUS",
            Self::UnknownRegistryRow => "RWT-REGISTRY-UNKNOWN",
            Self::InvalidPath => "RWT-PATH-INVALID",
            Self::UnsupportedEntry => "RWT-ENTRY-UNSUPPORTED",
            Self::ConcurrentMutation => "RWT-SESSION-MUTATED",
            Self::ContextMismatch => "RWT-CONTEXT-MISMATCH",
            Self::CapabilityUnavailable => "RWT-CAPABILITY-UNAVAILABLE",
            Self::CaptureFailed => "RWT-CAPTURE-FAILED",
            Self::CaptureLimit => "RWT-CAPTURE-LIMIT",
            Self::InvalidSnapshot => "RWT-SNAPSHOT-INVALID",
            Self::InvalidRequest => "RWT-REQUEST-INVALID",
            Self::InvalidReceipt => "RWT-RECEIPT-INVALID",
            Self::ObservationFailed => "RWT-OBSERVATION-FAILED",
            Self::Serialization => "RWT-SERIALIZATION-FAILED",
        }
    }
}

#[derive(Eq, PartialEq)]
pub struct RoutineError {
    id: RoutineErrorId,
    cause: &'static str,
    subject_sha256: Option<String>,
    process_custody: Option<Box<ProcessCustodyEvidence>>,
    launch_cleanup: Option<Box<LaunchCleanupEvidence>>,
    transition_failure: Option<Box<ReservationTransitionFailure>>,
}

impl RoutineError {
    pub(crate) fn new(
        id: RoutineErrorId,
        cause: &'static str,
        sensitive_subject: Option<&[u8]>,
    ) -> Self {
        let subject_sha256 =
            sensitive_subject.map(|value| format!("sha256:{:x}", Sha256::digest(value)));
        Self {
            id,
            cause,
            subject_sha256,
            process_custody: None,
            launch_cleanup: None,
            transition_failure: None,
        }
    }

    pub(crate) fn evidence(&self) -> ErrorEvidence {
        ErrorEvidence {
            code: self.code().to_owned(),
            cause: self.cause.to_owned(),
            subject_sha256: self.subject_sha256.clone(),
        }
    }

    pub(in crate::routine_work) fn with_process_custody(
        mut self,
        observation: ObservedProcessCustody,
    ) -> Self {
        self.process_custody = Some(Box::new(observation.into_evidence()));
        self
    }

    pub(crate) fn process_custody(&self) -> Option<&ProcessCustodyEvidence> {
        self.process_custody.as_deref()
    }

    pub(in crate::routine_work) fn with_launch_cleanup(
        mut self,
        observation: super::runtime_adapter::ObservedLaunchCleanup,
    ) -> Self {
        self.launch_cleanup = Some(Box::new(observation.into_evidence()));
        self
    }

    pub(in crate::routine_work) fn launch_cleanup(&self) -> Option<&LaunchCleanupEvidence> {
        self.launch_cleanup.as_deref()
    }

    fn with_transition_failure(mut self, failure: ReservationTransitionFailure) -> Self {
        self.transition_failure = Some(Box::new(failure));
        self
    }

    pub fn id(&self) -> RoutineErrorId {
        self.id
    }

    pub fn code(&self) -> &'static str {
        self.id.code()
    }

    pub fn cause(&self) -> &'static str {
        self.cause
    }

    pub fn subject_sha256(&self) -> Option<&str> {
        self.subject_sha256.as_deref()
    }
}

impl fmt::Display for RoutineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} cause={}", self.code(), self.cause)?;
        if let Some(subject) = &self.subject_sha256 {
            write!(formatter, " subject={subject}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for RoutineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl std::error::Error for RoutineError {}
