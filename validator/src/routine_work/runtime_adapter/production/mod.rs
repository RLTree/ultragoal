//! Sealed production issuance and durable routine mediation authority.

mod ledger;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::context::LiveContext;
use crate::routine_work::digest::{digest_of, framed};
use crate::routine_work::{
    CleanupEvidence, FailureEvidence, PanicEvidence, RESERVATION_FAILURE_SCHEMA,
    ReservationFailureDisposition, ReservationFailureEvidence, RoutineError, RoutineErrorId,
    RoutinePlan, transition_failure_error,
};

use super::execution_authority::{PreparedRoutineExecution, RoutineEffectRequest};
use super::mediator::{
    DurableSettlement, PreflightedProductionReuse, RoutineArtifactPublisher, mediate_noop,
    preflight_production_request, preflight_production_reuse_input, recovery_identity,
};
use super::{RoutineCancellation, RoutineMediationResult, RoutineReuseInput};
use ledger::{
    AttemptState, AuthorityBinding, FileAuthorityLedger, OutputProvisionJournal, ReservationSpec,
    ReservationToken, ReuseArtifactClaim,
};

#[path = "launch_custody.rs"]
mod launch_custody;
#[path = "launch_recovery.rs"]
mod launch_recovery;
#[path = "launch_root.rs"]
mod launch_root;
#[path = "launch_snapshot.rs"]
mod launch_snapshot;
#[path = "output_journal/mod.rs"]
mod output_journal;
#[path = "production_issuance.rs"]
mod production_issuance;
#[path = "production_mediation.rs"]
mod production_mediation;
#[path = "reservation_failure.rs"]
mod reservation_failure;
#[path = "reservation_transaction.rs"]
mod reservation_transaction;

use production_mediation::error;
pub(crate) use reservation_transaction::RoutineExecutionCapability;

/// Canonical public production entry. Issuer construction, reservation,
/// recovery lookup, and raw settlement stay inside this leaf.
pub(crate) fn mediate_public_routine_execution(
    authority_root: Option<&Path>,
    context: &LiveContext,
    plan: &RoutinePlan,
    prepared: PreparedRoutineExecution,
    cancellation: RoutineCancellation,
    reuse: RoutineReuseInput,
    publisher: Option<&dyn RoutineArtifactPublisher>,
) -> Result<RoutineMediationResult, RoutineError> {
    if matches!(prepared, PreparedRoutineExecution::NoOp(_)) {
        if !reuse.is_empty() || publisher.is_some() {
            return Err(error("routine-production-noop-authority-or-reuse-present"));
        }
        let PreparedRoutineExecution::NoOp(projection) = prepared else {
            return Err(error("routine-production-noop-required"));
        };
        return mediate_noop(context, plan, projection, reuse);
    }
    let PreparedRoutineExecution::Effect(request) = prepared else {
        return Err(error("routine-production-effect-required"));
    };
    preflight_production_request(context, plan, &request)?;
    let publisher = publisher.ok_or_else(|| error("routine-production-publisher-missing"))?;
    let reuse_supplied = !reuse.is_empty();
    let reuse = preflight_production_reuse_input(reuse, &request, reuse_supplied)?;
    let authority_root =
        authority_root.ok_or_else(|| error("routine-production-authority-root-missing"))?;
    reservation_transaction::mediate_reserved_effect(
        authority_root,
        context,
        plan,
        request,
        cancellation,
        reuse,
        publisher,
    )
}
