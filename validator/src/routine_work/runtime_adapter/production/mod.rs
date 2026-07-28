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

type ReservationPublicationCallback<'a> = dyn for<'publication> FnMut(&'publication RoutineReservationPublication) -> Result<(), RoutineError>
    + 'a;

/// Private controls carried together from production admission to the one
/// reservation-and-effect transition.
pub(crate) struct ProductionExecutionControl<'a> {
    custody: Option<RoutineCustodyCapability>,
    cancellation: RoutineCancellation,
    reuse: RoutineReuseInput,
    control: PublicRoutineControl,
    predecessor_continuations: &'a [String],
    on_reserved: Option<&'a mut ReservationPublicationCallback<'a>>,
}

pub(in crate::routine_work::runtime_adapter::production) struct ReservedEffectControl<'a> {
    cancellation: RoutineCancellation,
    reuse: PreflightedProductionReuse,
    control: PublicRoutineControl,
    predecessor_continuations: &'a [String],
    on_reserved: Option<&'a mut ReservationPublicationCallback<'a>>,
}

impl<'a> ProductionExecutionControl<'a> {
    pub(crate) fn standard(
        custody: Option<RoutineCustodyCapability>,
        control: PublicRoutineControl,
    ) -> Self {
        Self {
            custody,
            cancellation: RoutineCancellation::new(),
            reuse: RoutineReuseInput::default(),
            control,
            predecessor_continuations: &[],
            on_reserved: None,
        }
    }

    pub(crate) fn with_reservation_publication(
        custody: RoutineCustodyCapability,
        control: PublicRoutineControl,
        predecessor_continuations: &'a [String],
        on_reserved: &'a mut ReservationPublicationCallback<'a>,
    ) -> Self {
        Self {
            custody: Some(custody),
            cancellation: RoutineCancellation::new(),
            reuse: RoutineReuseInput::default(),
            control,
            predecessor_continuations,
            on_reserved: Some(on_reserved),
        }
    }
}

/// Private host checkpoint data published after the durable reservation and
/// before any workspace effect can begin.
pub(crate) struct RoutineReservationPublication {
    continuation: String,
    recovery_marker: String,
    attempt_grant: String,
    authenticated_ledger_head: String,
}

impl RoutineReservationPublication {
    pub(in crate::routine_work::runtime_adapter::production) fn new(
        continuation: String,
        recovery_marker: String,
        attempt_grant: String,
        authenticated_ledger_head: String,
    ) -> Self {
        Self {
            continuation,
            recovery_marker,
            attempt_grant,
            authenticated_ledger_head,
        }
    }

    pub(crate) fn continuation(&self) -> &str {
        &self.continuation
    }

    pub(crate) fn recovery_marker(&self) -> &str {
        &self.recovery_marker
    }

    pub(crate) fn attempt_grant(&self) -> &str {
        &self.attempt_grant
    }

    pub(crate) fn authenticated_ledger_head(&self) -> &str {
        &self.authenticated_ledger_head
    }
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
        context,
        plan,
        prepared,
        ProductionExecutionControl {
            custody,
            cancellation,
            reuse,
            control: PublicRoutineControl::Run,
            predecessor_continuations: &[],
            on_reserved: None,
        },
    )
}

#[cfg(test)]
pub(crate) fn mediate_public_routine_execution_with_reservation_publication<'a>(
    authority_root: &Path,
    context: &LiveContext,
    plan: &RoutinePlan,
    prepared: PreparedRoutineExecution,
    on_reserved: &'a mut ReservationPublicationCallback<'a>,
) -> Result<RoutineMediationResult, RoutineError> {
    mediate_public_routine_execution_with_control(
        context,
        plan,
        prepared,
        ProductionExecutionControl::with_reservation_publication(
            custody::issue_test_custody(authority_root),
            PublicRoutineControl::Run,
            &[],
            on_reserved,
        ),
    )
}

pub(crate) fn mediate_public_routine_execution_with_control(
    context: &LiveContext,
    plan: &RoutinePlan,
    prepared: PreparedRoutineExecution,
    control: ProductionExecutionControl<'_>,
) -> Result<RoutineMediationResult, RoutineError> {
    let ProductionExecutionControl {
        custody,
        cancellation,
        reuse,
        control,
        predecessor_continuations,
        on_reserved,
    } = control;
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
    let control = ReservedEffectControl {
        cancellation,
        reuse,
        control,
        predecessor_continuations,
        on_reserved,
    };
    custody::mediate_reserved_effect(custody, context, plan, request, control)
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

/// Authenticates a public host checkpoint against the private HMAC ledger.
///
/// The host checkpoint is intentionally only a projection.  This read-only
/// path opens an already-created custody store and verifies the exact attempt
/// binding before a caller may project, append a terminal event, or advance
/// the host projection.  `allow_stale_head` is reserved for a one-shot
/// continuation alias: the private attempt and binding still must match, but
/// its old projection head may have been superseded by the handoff attempt.
/// A completed terminal checkpoint may also replay after an independently
/// bound attempt advances the shared ledger; it cannot advance custody.
pub(crate) fn authenticate_public_routine_checkpoint(
    custody: RoutineCustodyCapability,
    target: &Path,
    context_id: &str,
    candidate_id: &str,
    plan_id: &str,
    snapshot_id: &str,
    continuation: &str,
    recovery_marker: &str,
    predecessor_continuations: &[String],
    attempt_grant: &str,
    authenticated_ledger_head: &str,
    state: &str,
    terminal_outcome: Option<&str>,
    allow_stale_head: bool,
) -> Result<(), RoutineError> {
    custody::authenticate_public_checkpoint(
        custody,
        target,
        context_id,
        candidate_id,
        plan_id,
        snapshot_id,
        continuation,
        recovery_marker,
        predecessor_continuations,
        attempt_grant,
        authenticated_ledger_head,
        state,
        terminal_outcome,
        allow_stale_head,
    )
}
