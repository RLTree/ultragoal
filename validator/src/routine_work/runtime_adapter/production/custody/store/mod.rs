//! Descriptor-bound durable authority for production routine mediation.

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::routine_work::digest::{canonical, sha256, valid};
use crate::routine_work::{
    CleanupEvidence, ReservationFailureDisposition, ReservationFailureEvidence, RoutineError,
    RoutineErrorId,
};

#[path = "authority_record.rs"]
mod authority_record;
#[path = "file_authority.rs"]
mod file_authority;
#[path = "file_authority_failure.rs"]
mod file_authority_failure;
#[cfg(target_vendor = "apple")]
#[path = "supported/mod.rs"]
mod supported;

pub(crate) use authority_record::*;
pub(crate) use file_authority::*;
#[cfg(target_vendor = "apple")]
pub(in crate::routine_work::runtime_adapter::production::custody) use supported::LocalHead;

pub(in crate::routine_work::runtime_adapter::production::custody) enum DurableWrite<T> {
    Committed(T),
    Precommit(T),
    Ambiguous(T),
}

impl<T> DurableWrite<T> {
    pub(crate) fn into_result(self) -> Result<T, RoutineError> {
        match self {
            Self::Committed(value) => Ok(value),
            Self::Precommit(_) => Err(error("routine-production-authority-publish-precommit")),
            Self::Ambiguous(_) => Err(error("routine-production-authority-publish-ambiguous")),
        }
    }
}
