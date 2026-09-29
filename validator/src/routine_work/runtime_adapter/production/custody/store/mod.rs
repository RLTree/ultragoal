//! Descriptor-bound durable authority for production routine mediation.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
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
mod observation;
#[cfg(target_vendor = "apple")]
mod supported;

use authority_record::*;
pub(in crate::routine_work::runtime_adapter::production) use authority_record::{
    AuthorityBinding, OutputComponentJournal, OutputDirectoryIdentity, OutputProvisionJournal,
    OutputStageAmbiguity,
};
pub(in crate::routine_work::runtime_adapter::production::custody) use authority_record::{
    TerminalMediation, TerminalNodeMediation,
};
pub(in crate::routine_work::runtime_adapter::production::custody) use file_authority::DurableCustody;
use file_authority::error;
#[cfg(target_vendor = "apple")]
pub(super) use supported::reserved_reconciliation::ContinuationDisposition;
#[cfg(all(test, target_vendor = "apple"))]
pub(crate) use supported::{
    set_test_publication_ambiguity_after, set_test_publication_refusal_after,
};

pub(in crate::routine_work::runtime_adapter::production::custody) enum DurableWrite<T> {
    Committed(T),
    Precommit(T),
    Ambiguous(T, DurableAmbiguity),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::routine_work::runtime_adapter::production::custody) struct DurableAmbiguity {
    pub(in crate::routine_work::runtime_adapter::production::custody) previous_head_sha256: String,
    pub(in crate::routine_work::runtime_adapter::production::custody) proposed_head_sha256: String,
}

impl<T> DurableWrite<T> {
    pub(crate) fn into_result(self) -> Result<T, RoutineError> {
        match self {
            Self::Committed(value) => Ok(value),
            Self::Precommit(_) => Err(error("routine-production-authority-publish-precommit")),
            Self::Ambiguous(_, _) => Err(error("routine-production-authority-publish-ambiguous")),
        }
    }
}
