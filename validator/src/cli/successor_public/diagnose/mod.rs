use super::local_store::{LocalStore, RUNTIME_SOURCE_ID, routine_observations_from_events};
use super::public_output_allowed;
use crate::cli::successor::runtime::{Diagnostic, DiagnosticDetails, DiagnosticId, RuntimeOutcome};
use crate::cli::successor::{
    EffectClass, ExitClass, OptionName, ParsedInvocation, ParsedValue, SuccessorCommand,
};
use crate::context::LiveContext;
use crate::observability::{EventQuery, EventStore};
use crate::state::{
    Finding, ProductState, RoutineFindingBinding, RoutineFindingObservation,
    RoutineObservationWindow,
};
use serde_json::Value;
use std::path::Path;

mod causal;
mod request;
#[path = "routine_next.rs"]
mod routine_next_projection;
#[path = "routine.rs"]
mod routine_projection;

pub(super) use request::{request, requests_target};

pub(super) fn diagnose_routine(
    root: &Path,
    context: &LiveContext,
    invocation: &ParsedInvocation,
    home: Option<&Path>,
) -> RuntimeOutcome {
    routine_projection::diagnose(root, context, invocation, home, None)
}

pub(super) fn diagnose_routine_or(
    root: &Path,
    context: &LiveContext,
    invocation: &ParsedInvocation,
    home: Option<&Path>,
    fallback: RuntimeOutcome,
) -> RuntimeOutcome {
    routine_projection::diagnose(root, context, invocation, home, Some(fallback))
}

pub(super) fn next_routine_or(
    root: &Path,
    context: &LiveContext,
    invocation: &ParsedInvocation,
    home: Option<&Path>,
    fallback: RuntimeOutcome,
) -> RuntimeOutcome {
    routine_next_projection::project(root, context, invocation, home, fallback)
}

pub(super) fn diagnose_local(
    root: &Path,
    context: &LiveContext,
    mut state: ProductState,
    invocation: &ParsedInvocation,
    home: Option<&Path>,
) -> RuntimeOutcome {
    if invocation.command != SuccessorCommand::Diagnose || invocation.effect != EffectClass::Read {
        return invalid_invocation();
    }
    let request = match request(invocation) {
        Ok(request) if request.target.is_none() => request,
        _ => return invalid_invocation(),
    };
    let (routine_observations, routine_window) =
        read_routine_observations(home, root, context, &state);
    state.attach_routine_observations(routine_observations, routine_window);
    let selected = match request.finding {
        Some(identifier) => match select_finding(&state, identifier) {
            Some(finding) => Some(finding),
            None => return finding_not_present(),
        },
        None => None,
    };
    let diagnosis = match selected {
        Some(finding) => state.diagnose_finding_json(&finding.finding_id),
        None => state.diagnose_json().map(Some),
    };
    let Some(diagnosis) = diagnosis.ok().flatten() else {
        return diagnosis_unavailable();
    };
    let mut value: Value = match serde_json::from_slice(&diagnosis) {
        Ok(value @ Value::Object(_)) => value,
        _ => return diagnosis_unavailable(),
    };
    let observability = match selected {
        Some(finding) => causal::evaluate(home, root, context, finding),
        None => causal::not_evaluated(),
    };
    let Value::Object(object) = &mut value else {
        unreachable!()
    };
    object.insert("observability".to_owned(), observability);
    object.insert("claim_effect".to_owned(), Value::String("none".to_owned()));
    if context.revalidate().is_err() {
        return stale_context();
    }
    match serde_json::to_vec(&value) {
        Ok(machine) if public_output_allowed(machine.len()) => RuntimeOutcome::payload(
            if state.findings().is_empty() {
                ExitClass::Success
            } else {
                ExitClass::ActionableFinding
            },
            machine,
            format!(
                "diagnosis state={} findings={} claim_effect=none",
                state.state_id(),
                state.findings().len()
            ),
        ),
        _ => diagnosis_unavailable(),
    }
}

fn read_routine_observations(
    home: Option<&Path>,
    root: &Path,
    context: &LiveContext,
    state: &ProductState,
) -> (Vec<RoutineFindingObservation>, RoutineObservationWindow) {
    let store = match LocalStore::open(home, root, context, RUNTIME_SOURCE_ID) {
        Ok(store) if store.status() == "available" => store,
        Ok(_) => return (Vec::new(), RoutineObservationWindow::Absent),
        Err(_) => return (Vec::new(), RoutineObservationWindow::Unavailable),
    };
    let query = match EventQuery::for_context(context, RUNTIME_SOURCE_ID)
        .and_then(|query| query.limit(EventStore::supported_result_limit()))
    {
        Ok(query) => query,
        Err(_) => return (Vec::new(), RoutineObservationWindow::Unavailable),
    };
    let events = match store.query_diagnostic(&query) {
        Ok(events) => events,
        Err(_) => return (Vec::new(), RoutineObservationWindow::Unavailable),
    };
    if store.revalidate().is_err() {
        return (Vec::new(), RoutineObservationWindow::Unavailable);
    }
    if events.len() == EventStore::supported_result_limit() {
        return (Vec::new(), RoutineObservationWindow::Saturated);
    }
    let bound_events = events
        .iter()
        .filter(|event| {
            state.findings().iter().any(|finding| {
                event.finding_refs().contains(&finding.finding_id)
                    && event.repair_refs().contains(&finding.repair.repair_id)
            })
        })
        .cloned()
        .collect::<Vec<_>>();
    let observations = routine_observations_from_events(&bound_events)
        .into_iter()
        .filter(|observation| {
            let binding = RoutineFindingBinding {
                finding_id: observation.finding_id.clone(),
                repair_id: observation.repair_id.clone(),
            };
            state
                .findings()
                .iter()
                .any(|finding| binding.matches(finding))
        })
        .collect();
    (observations, RoutineObservationWindow::Available)
}

fn select_finding<'a>(state: &'a ProductState, identifier: &str) -> Option<&'a Finding> {
    let mut exact = state
        .findings()
        .iter()
        .filter(|finding| finding.finding_id == identifier);
    match (exact.next(), exact.next()) {
        (Some(finding), None) => return Some(finding),
        (None, None) => {}
        _ => return None,
    }
    let mut by_code = state
        .findings()
        .iter()
        .filter(|finding| finding.code == identifier);
    match (by_code.next(), by_code.next()) {
        (Some(finding), None) => Some(finding),
        _ => None,
    }
}

fn invalid_invocation() -> RuntimeOutcome {
    failure(
        DiagnosticId::UnexpectedArguments,
        ExitClass::InvalidInvocation,
        "diagnose received arguments outside the typed route contract",
        "invoke diagnose with at most one typed --finding identifier",
        "diagnosis and dependent claims remain unchanged",
    )
}

fn finding_not_present() -> RuntimeOutcome {
    failure(
        DiagnosticId::FindingNotPresent,
        ExitClass::ActionableFinding,
        "requested finding is not present in the current state graph",
        "re-run inspect findings and select one current finding identifier",
        "runtime and observability claims remain current-state-only",
    )
}

fn diagnosis_unavailable() -> RuntimeOutcome {
    failure(
        DiagnosticId::StateUnavailable,
        ExitClass::UnsupportedCapability,
        "the bounded diagnosis projection could not be compiled",
        "repair the current state projection and rerun diagnosis",
        "diagnosis and dependent claims remain withheld",
    )
}

fn routine_checkpoint_failure(
    failure: super::routine::RoutineCheckpointFailure,
    rerun: &'static str,
) -> RuntimeOutcome {
    let (id, class, cause, repair) = match failure {
        super::routine::RoutineCheckpointFailure::Host(
            super::routine::HostFailure::Unavailable,
        ) => (
            DiagnosticId::AuthorityRequired,
            ExitClass::BlockedAuthority,
            "the owner-only routine state authority is unavailable",
            "install or repair the exact owner-only routine authority, then retry the same read request",
        ),
        super::routine::RoutineCheckpointFailure::Host(super::routine::HostFailure::Busy) => (
            DiagnosticId::AuthorityRequired,
            ExitClass::BlockedAuthority,
            "the exact routine state authority is busy with another invocation",
            "wait for the active invocation to settle, then retry the same read request",
        ),
        super::routine::RoutineCheckpointFailure::Host(
            super::routine::HostFailure::TransitionAmbiguous,
        ) => (
            DiagnosticId::StateUnavailable,
            ExitClass::ActionableFinding,
            "a prior routine HostState namespace transition has uncertain durability or verification",
            "preserve every transition artifact and diagnose again only after the HostState owner settles",
        ),
        super::routine::RoutineCheckpointFailure::Host(super::routine::HostFailure::Invalid) => (
            DiagnosticId::AuthorityRequired,
            ExitClass::BlockedAuthority,
            "the routine state authority is stale, malformed, substituted, or unsafe",
            "preserve the authority bytes and repair or update the owning routine adapter before retrying",
        ),
        #[cfg(not(target_vendor = "apple"))]
        super::routine::RoutineCheckpointFailure::Host(
            super::routine::HostFailure::Unsupported,
        ) => (
            DiagnosticId::DownstreamToolUnavailable,
            ExitClass::UnsupportedCapability,
            "routine state projection is unavailable on this host",
            "run the request on the supported Darwin host runtime",
        ),
        super::routine::RoutineCheckpointFailure::Projection => (
            DiagnosticId::StateUnavailable,
            ExitClass::UnsupportedCapability,
            "the current routine checkpoint projection could not be derived",
            "repair the current routine source projection and retry the same read request",
        ),
        super::routine::RoutineCheckpointFailure::StaleContext => (
            DiagnosticId::StaleContext,
            ExitClass::ActionableFinding,
            "the live routine candidate changed while deriving the checkpoint",
            "recompute current context and retry the same read request",
        ),
    };
    RuntimeOutcome::failure(
        class,
        Diagnostic::new(
            id,
            class,
            DiagnosticDetails {
                cause,
                affected_surface: "routine checkpoint navigation",
                repair,
                effect: "read",
                rerun,
                ceiling: "routine diagnosis and next-action claims remain withheld",
            },
        ),
    )
}

fn stale_context() -> RuntimeOutcome {
    failure(
        DiagnosticId::StaleContext,
        ExitClass::ActionableFinding,
        "the live candidate changed during diagnosis",
        "recompute current context and rerun diagnosis",
        "same-candidate diagnosis and dependent claims remain withheld",
    )
}

fn failure(
    id: DiagnosticId,
    class: ExitClass,
    cause: &'static str,
    repair: &'static str,
    ceiling: &'static str,
) -> RuntimeOutcome {
    RuntimeOutcome::failure(
        class,
        Diagnostic::new(
            id,
            class,
            DiagnosticDetails {
                cause,
                affected_surface: "HCT-STATE + HCT-OBSERVE diagnosis",
                repair,
                effect: "read",
                rerun: "ultragoal --json diagnose",
                ceiling,
            },
        ),
    )
}
