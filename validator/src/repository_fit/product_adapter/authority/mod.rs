//! Sealed production repository-fit authority candidate.
//!
//! The public dispatcher and host policy remain root-owned. This module is the
//! sole high-level source entry that may initialize the durable authority store,
//! reserve an effect, activate `LocalEffects`, and settle a terminal record.

use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};
use std::path::Path;

#[cfg(test)]
use std::cell::RefCell;

use crate::context::LiveContext;
use crate::repository_fit::local::LocalEffects;
use crate::repository_fit::{
    CanonicalPath, ExpectedContent, PreparedFitApply, digest, valid_digest,
};

use super::ledger::{
    EffectOwner, FileRepositoryFitLedger, LedgerError, LedgerErrorId, RECOVERY_INTENT_SCHEMA,
    RecoveryTargetRow, RecoveryTargetSpec, RecoveryTerminal, RepositoryFitLedgerState,
    ReservationDecision, ReservationRequest, ReservationToken, canonical_recovery_intent_bytes,
};
use super::root_permit::{
    MAX_PERMIT_LIFETIME, MIN_NONCE_BYTES, ManagedAncestorContract, RepositoryFitApplyFailure,
    RepositoryFitApplyOutcome, activate_production_permit, apply_with_root_permit,
    observe_recovery_target_contract, prepare_production_permit,
    prepare_recovery_managed_ancestor_contract, prepared_managed_ancestor_contract,
    production_authority_identity, production_reservation_binding,
};
use super::{AdapterErrorId, FitAdapterError, adapter_error};

#[path = "effect_race_hook.rs"]
mod effect_race_hook;
#[path = "permit_lifetime.rs"]
mod permit_lifetime;
#[path = "production_outcome.rs"]
mod production_outcome;
#[path = "recovery_execution.rs"]
mod recovery_execution;
#[path = "recovery_target.rs"]
mod recovery_target;
#[path = "sealed_authority.rs"]
mod sealed_authority;
#[path = "supported_execution.rs"]
mod supported_execution;
#[path = "terminal_binding.rs"]
mod terminal_binding;

pub(crate) use effect_race_hook::*;
pub(crate) use permit_lifetime::*;
pub(crate) use recovery_execution::*;
pub(crate) use recovery_target::*;
pub(crate) use sealed_authority::*;
pub(crate) use supported_execution::*;
pub(crate) use terminal_binding::*;
