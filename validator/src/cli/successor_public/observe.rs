#[cfg(test)]
pub(super) use super::local_store::store_path;
use super::local_store::{
    LocalStore, LocalStoreFailure, local_policy, routine_observations_from_events,
};
use super::public_output_allowed;
use crate::cli::successor::runtime::{Diagnostic, DiagnosticDetails, DiagnosticId, RuntimeOutcome};
use crate::cli::successor::{
    EffectClass, ExitClass, ObserveAction, OptionName, ParsedInvocation, ParsedValue,
    SuccessorCommand,
};
use crate::context::LiveContext;
use crate::observability::{EventQuery, EventStore, SemanticEvent};
use serde_json::json;
use std::path::Path;

const SOURCE_ID: &str = "successor-runtime";
pub(super) fn query_local(
    root: &Path,
    context: &LiveContext,
    invocation: &ParsedInvocation,
) -> RuntimeOutcome {
    if invocation.command != SuccessorCommand::Observe(ObserveAction::Query)
        || invocation.effect != EffectClass::Read
    {
        return invalid_invocation();
    }
    let filter = match invocation.arguments.as_slice() {
        [] => None,
        [argument]
            if argument.name == OptionName::Filter
                && matches!(argument.value, ParsedValue::Identifier(_)) =>
        {
            let ParsedValue::Identifier(value) = &argument.value else {
                unreachable!()
            };
            Some(value.as_str())
        }
        _ => return invalid_invocation(),
    };
    if context.revalidate().is_err() {
        return stale_context();
    }
    let binding = match SemanticEvent::for_context(
        context,
        SOURCE_ID,
        "query-binding",
        0,
        0,
        "observe.query",
        "unknown",
    ) {
        Ok(binding) => binding,
        Err(_) => return observability_unavailable(LocalStoreFailure::binding()),
    };
    let mut query = match EventQuery::for_context(context, SOURCE_ID)
        .and_then(|query| query.limit(EventStore::supported_result_limit()))
    {
        Ok(query) => query,
        Err(_) => return observability_unavailable(LocalStoreFailure::binding()),
    };
    if let Some(operation) = filter {
        query = match query.operation(operation) {
            Ok(query) => query,
            Err(_) => return invalid_invocation(),
        };
    }
    let store = match LocalStore::open(root, context, SOURCE_ID) {
        Ok(store) => store,
        Err(failure) => return observability_unavailable(failure),
    };
    let events = match store.query_diagnostic(&query) {
        Ok(events) => events,
        Err(failure) if failure.is_lock_timeout() => {
            return observability_lock_timeout();
        }
        Err(failure) => return observability_unavailable(failure),
    };
    if let Err(failure) = store.revalidate() {
        return observability_unavailable(failure);
    }
    if context.revalidate().is_err() {
        return stale_context();
    }
    let count = events.len();
    let store_status = store.status();
    let machine = serde_json::to_vec(&json!({
        "schema_version": "ObservabilityQuery-v1",
        "context_id": binding.context_id(),
        "candidate_id": binding.candidate_id(),
        "source_id": binding.source_id(),
        "filter": filter,
        "store_status": store_status,
        "event_count": count,
        "routine_observations": routine_observations_from_events(&events),
        "events": events,
        "causal_status": "not_evaluated",
        "claim_effect": "none",
        "query_provenance": {
            "schema_version": "LocalQueryProvenance-v1",
            "binding": "context-candidate-source",
            "ordering": "observed_at_unix_ms-sequence-event_id",
            "result_count": count,
            "result_limit": EventStore::supported_result_limit(),
            "result_window_saturated": count == EventStore::supported_result_limit(),
            "read_effect": "none",
            "export_effect": "none"
        },
        "local_policy": local_policy()
    }));
    match machine {
        Ok(machine) if public_output_allowed(machine.len()) => RuntimeOutcome::payload(
            ExitClass::Success,
            machine,
            format!("local semantic events count={count} claim_effect=none"),
        ),
        _ => observability_unavailable(LocalStoreFailure::projection()),
    }
}

fn invalid_invocation() -> RuntimeOutcome {
    RuntimeOutcome::failure(
        ExitClass::InvalidInvocation,
        Diagnostic::new(
            DiagnosticId::UnexpectedArguments,
            ExitClass::InvalidInvocation,
            DiagnosticDetails {
                cause: "observe query received arguments outside the typed route contract",
                affected_surface: "HCT-OBSERVE query adapter",
                repair: "invoke observe query with at most one typed --filter identifier",
                effect: "read",
                rerun: "ultragoal --json observe query",
                ceiling: "observability and dependent claims remain unchanged",
            },
        ),
    )
}

fn stale_context() -> RuntimeOutcome {
    RuntimeOutcome::failure(
        ExitClass::ActionableFinding,
        Diagnostic::new(
            DiagnosticId::StaleContext,
            ExitClass::ActionableFinding,
            DiagnosticDetails {
                cause: "the live candidate changed during the local observability query",
                affected_surface: "HCT-OBSERVE query adapter",
                repair: "rebuild one LiveContext and query the separately bound current-candidate store",
                effect: "read",
                rerun: "ultragoal --json observe query",
                ceiling: "same-candidate observability and dependent claims are withheld",
            },
        ),
    )
}

fn observability_unavailable(failure: LocalStoreFailure) -> RuntimeOutcome {
    RuntimeOutcome::failure(
        ExitClass::UnsupportedCapability,
        Diagnostic::new(
            DiagnosticId::ObservabilityUnavailable,
            ExitClass::UnsupportedCapability,
            DiagnosticDetails {
                cause: failure.summary(),
                affected_surface: failure.surface(),
                repair: failure.repair(),
                effect: "read",
                rerun: "ultragoal --json observe query",
                ceiling: "observability and dependent claims remain withheld",
            },
        ),
    )
}

fn observability_lock_timeout() -> RuntimeOutcome {
    RuntimeOutcome::failure(
        ExitClass::ActionableFinding,
        Diagnostic::new(
            DiagnosticId::ObservabilityUnavailable,
            ExitClass::ActionableFinding,
            DiagnosticDetails {
                cause: "the bounded local event store lock deadline expired before a stable query could begin",
                affected_surface: "HCT-OBSERVE local store lock",
                repair: "retry after the current local writer finishes or diagnose the process holding the confined store lock",
                effect: "read",
                rerun: "ultragoal --json observe query",
                ceiling: "observability and dependent claims remain withheld until one bounded query succeeds",
            },
        ),
    )
}
