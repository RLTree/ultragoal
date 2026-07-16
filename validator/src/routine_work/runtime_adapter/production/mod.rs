//! Sealed production issuance and durable routine mediation authority.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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
    DurableSettlement, PreflightedProductionReuse, mediate_noop, preflight_production_request,
    preflight_production_reuse_input, recovery_identity,
};
use super::{RoutineCancellation, RoutineMediationResult, RoutineReuseInput};
#[path = "custody/mod.rs"]
mod custody;
#[path = "launch_custody/mod.rs"]
mod launch_custody;
#[path = "output_journal/mod.rs"]
mod output_journal;
#[path = "production_mediation.rs"]
mod production_mediation;
#[path = "reservation_failure.rs"]
mod reservation_failure;

use custody::AuthorityBinding;
#[cfg(all(test, target_vendor = "apple"))]
pub(crate) use custody::{
    set_test_publication_ambiguity_after, set_test_publication_refusal_after,
};
pub(in crate::routine_work) use launch_custody::ObservedStagedCleanup;
use production_mediation::error;

/// Canonical public production entry. Issuer construction, reservation,
/// recovery lookup, and raw settlement stay inside this leaf.
pub(crate) fn mediate_public_routine_execution(
    authority_root: Option<&Path>,
    context: &LiveContext,
    plan: &RoutinePlan,
    prepared: PreparedRoutineExecution,
    cancellation: RoutineCancellation,
    reuse: RoutineReuseInput,
) -> Result<RoutineMediationResult, RoutineError> {
    if matches!(prepared, PreparedRoutineExecution::NoOp(_)) {
        if !reuse.is_empty() {
            return Err(error("routine-production-noop-reuse-present"));
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
    let reuse_supplied = !reuse.is_empty();
    let reuse = preflight_production_reuse_input(reuse, &request, reuse_supplied)?;
    let authority_root =
        authority_root.ok_or_else(|| error("routine-production-authority-root-missing"))?;
    custody::mediate_reserved_effect(authority_root, context, plan, request, cancellation, reuse)
}
