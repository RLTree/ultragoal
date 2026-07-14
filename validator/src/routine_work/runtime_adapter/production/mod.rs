//! Sealed production issuance and durable routine mediation authority.

mod ledger;

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(test)]
use std::cell::RefCell;

use serde::Serialize;

use crate::context::LiveContext;
use crate::routine_work::digest::{digest_of, framed};
use crate::routine_work::{RoutineError, RoutineErrorId, RoutinePlan};

use super::execution_authority::{PreparedRoutineExecution, RoutineEffectRequest};
use super::mediator::{
    DurableAttemptAuthority, DurableSettlement, PreflightedProductionReuse, ProductionGrantBinding,
    issue_production_grant, mediate_prepared_routine_execution, preflight_production_request,
    preflight_production_reuse_input, production_grant_identity, production_recovery_identity,
};
use super::{RoutineCancellation, RoutineMediationResult, RoutineReuseInput};
use ledger::{
    AttemptState, AuthorityBinding, FileAuthorityLedger, ReservationSpec, ReservationToken,
    ReuseArtifactClaim,
};

#[path = "production_issuance.rs"]
mod production_issuance;
#[path = "production_mediation.rs"]
mod production_mediation;
#[path = "recovery_authority.rs"]
mod recovery_authority;

pub(crate) use production_mediation::*;
pub(crate) use recovery_authority::*;
