use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::any::Any;

use super::RoutineError;

pub(crate) const RESERVATION_FAILURE_SCHEMA: &str = "RoutineReservationFailure-v1";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ErrorEvidence {
    pub(crate) code: String,
    pub(crate) cause: String,
    pub(crate) subject_sha256: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PanicPayloadKind {
    StaticStr,
    String,
    OpaqueType,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PanicEvidence {
    pub(crate) payload_kind: PanicPayloadKind,
    pub(crate) payload_sha256: String,
    pub(crate) payload_byte_length: u64,
}

impl PanicEvidence {
    pub(crate) fn capture(payload: &(dyn Any + Send)) -> Self {
        if let Some(value) = payload.downcast_ref::<&'static str>() {
            return Self::observed(
                PanicPayloadKind::StaticStr,
                value.as_bytes(),
                value.len() as u64,
            );
        }
        if let Some(value) = payload.downcast_ref::<String>() {
            return Self::observed(
                PanicPayloadKind::String,
                value.as_bytes(),
                value.len() as u64,
            );
        }
        let type_identity = format!("{:?}", payload.type_id());
        Self::observed(PanicPayloadKind::OpaqueType, type_identity.as_bytes(), 0)
    }

    fn observed(kind: PanicPayloadKind, bytes: &[u8], payload_byte_length: u64) -> Self {
        Self {
            payload_kind: kind,
            payload_sha256: digest(bytes),
            payload_byte_length,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "evidence")]
pub(crate) enum FailureEvidence {
    MissingSettlement,
    Error(ErrorEvidence),
    Panic(PanicEvidence),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status", content = "evidence")]
pub(crate) enum CleanupEvidence {
    NotRequired,
    Succeeded,
    Error(ErrorEvidence),
    Panic(PanicEvidence),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProcessCustodyEvidence {
    pub(crate) primary: FailureEvidence,
    pub(crate) cleanup: CleanupEvidence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReservationFailureDisposition {
    ReservedPending,
    StartedPending,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReservationFailureEvidence {
    pub(crate) schema_version: String,
    pub(crate) protocol_id: String,
    pub(crate) grant_id: String,
    pub(crate) recovery_marker: String,
    pub(crate) primary: FailureEvidence,
    pub(crate) process_cleanup: CleanupEvidence,
    pub(crate) staged_cleanup: CleanupEvidence,
    pub(crate) disposition: ReservationFailureDisposition,
}

impl ReservationFailureEvidence {
    pub(crate) fn shape_is_valid(&self) -> bool {
        self.schema_version == RESERVATION_FAILURE_SCHEMA
            && valid_digest(&self.protocol_id)
            && valid_digest(&self.grant_id)
            && valid_digest(&self.recovery_marker)
            && failure_is_valid(&self.primary)
            && cleanup_is_valid(&self.process_cleanup)
            && cleanup_is_valid(&self.staged_cleanup)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReservationTransitionFailure {
    pub(crate) attempted: ReservationFailureEvidence,
    pub(crate) transition_failure: FailureEvidence,
}

fn failure_is_valid(value: &FailureEvidence) -> bool {
    match value {
        FailureEvidence::MissingSettlement => true,
        FailureEvidence::Error(error) => error_is_valid(error),
        FailureEvidence::Panic(panic) => panic_is_valid(panic),
    }
}

fn cleanup_is_valid(value: &CleanupEvidence) -> bool {
    match value {
        CleanupEvidence::NotRequired | CleanupEvidence::Succeeded => true,
        CleanupEvidence::Error(error) => error_is_valid(error),
        CleanupEvidence::Panic(panic) => panic_is_valid(panic),
    }
}

fn error_is_valid(value: &ErrorEvidence) -> bool {
    semantic(&value.code)
        && semantic(&value.cause)
        && value
            .subject_sha256
            .as_ref()
            .is_none_or(|subject| valid_digest(subject))
}

fn panic_is_valid(value: &PanicEvidence) -> bool {
    valid_digest(&value.payload_sha256) && value.payload_byte_length <= 16 * 1024
}

fn semantic(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn valid_digest(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub(crate) fn transition_failure_error(
    cause: &'static str,
    attempted: ReservationFailureEvidence,
    transition_failure: FailureEvidence,
) -> RoutineError {
    RoutineError::new(super::RoutineErrorId::InvalidRequest, cause, None).with_transition_failure(
        ReservationTransitionFailure {
            attempted,
            transition_failure,
        },
    )
}
