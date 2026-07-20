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
    DurableSettlement, PreflightedProductionReuse, RoutineContinuationOutcome, mediate_noop,
    preflight_production_request, preflight_production_reuse_input, recovery_identity,
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
#[path = "reservation_failure/mod.rs"]
mod reservation_failure;

use custody::AuthorityBinding;
pub(crate) use custody::RoutineCustodyCapability;
#[cfg(all(test, target_vendor = "apple"))]
pub(crate) use custody::{
    set_test_publication_ambiguity_after, set_test_publication_refusal_after,
};
pub(in crate::routine_work) use launch_custody::{LaunchCleanupEvidence, ObservedLaunchCleanup};
#[cfg(all(test, target_vendor = "apple"))]
pub(crate) use launch_custody::{
    set_test_launch_cleanup_refusal, set_test_launch_panic_after_stat,
    set_test_launch_stat_failure_after,
};
use production_mediation::error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PublicRoutineControl {
    Run,
    InterruptAfterReservation,
}

/// Canonical public production entry. Issuer construction, reservation,
/// recovery lookup, and raw settlement stay inside this leaf.
#[cfg(test)]
pub(crate) fn mediate_public_routine_execution(
    authority_root: Option<&Path>,
    context: &LiveContext,
    plan: &RoutinePlan,
    prepared: PreparedRoutineExecution,
    cancellation: RoutineCancellation,
    reuse: RoutineReuseInput,
) -> Result<RoutineMediationResult, RoutineError> {
    let custody = authority_root.map(custody::issue_test_custody);
    mediate_public_routine_execution_with_control(
        custody,
        context,
        plan,
        prepared,
        cancellation,
        reuse,
        PublicRoutineControl::Run,
    )
}

pub(crate) fn mediate_public_routine_execution_with_control(
    custody: Option<RoutineCustodyCapability>,
    context: &LiveContext,
    plan: &RoutinePlan,
    prepared: PreparedRoutineExecution,
    cancellation: RoutineCancellation,
    reuse: RoutineReuseInput,
    control: PublicRoutineControl,
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
    let custody = custody.ok_or_else(|| error("routine-production-authority-root-missing"))?;
    custody::mediate_reserved_effect(
        custody,
        context,
        plan,
        request,
        cancellation,
        reuse,
        control,
    )
}

pub(crate) fn reconcile_public_routine_reservation(
    custody: RoutineCustodyCapability,
    context: &LiveContext,
    plan: &RoutinePlan,
    prepared: PreparedRoutineExecution,
    attempt_grant: &str,
    expected_head: &str,
) -> Result<RoutineContinuationOutcome, RoutineError> {
    let PreparedRoutineExecution::Effect(request) = prepared else {
        return Err(error("routine-production-continuation-effect-required"));
    };
    preflight_production_request(context, plan, &request)?;
    custody::reconcile_reserved_effect(custody, &request, attempt_grant, expected_head)
}
