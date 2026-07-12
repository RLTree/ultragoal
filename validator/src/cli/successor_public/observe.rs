use super::local_store::LocalStore;
pub(super) use super::local_store::store_path;
use crate::cli::successor::runtime::{Diagnostic, DiagnosticId, RuntimeOutcome};
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
        Err(_) => return observability_unavailable(),
    };
    let mut query = match EventQuery::for_context(context, SOURCE_ID) {
        Ok(query) => query,
        Err(_) => return observability_unavailable(),
    };
    if let Some(operation) = filter {
        query = match query.operation(operation) {
            Ok(query) => query,
            Err(_) => return invalid_invocation(),
        };
    }
    let store = match LocalStore::open(root, context, SOURCE_ID) {
        Ok(store) => store,
        Err(()) => return observability_unavailable(),
    };
    let events = match store.query(&query) {
        Ok(events) => events,
        Err(()) => return observability_unavailable(),
    };
    if !store.revalidate() {
        return observability_unavailable();
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
        "events": events,
        "causal_status": "not_evaluated",
        "claim_effect": "none",
        "local_policy": {
            "schema_version": "LocalObservabilityPolicy-v1",
            "store_limit_bytes": EventStore::supported_store_limit_bytes(),
            "event_limit": EventStore::supported_event_limit(),
            "scan_row_limit": EventStore::supported_scan_limit(),
            "query_result_limit": EventStore::supported_result_limit(),
            "deletion": "explicit-clear-api",
            "external_export": "disabled-safe-default-OD-004-OD-007"
        }
    }));
    match machine {
        Ok(machine) => RuntimeOutcome::payload(
            ExitClass::Success,
            machine,
            format!("local semantic events count={count} claim_effect=none"),
        ),
        Err(_) => observability_unavailable(),
    }
}

fn invalid_invocation() -> RuntimeOutcome {
    RuntimeOutcome::failure(
        ExitClass::InvalidInvocation,
        Diagnostic::new(
            DiagnosticId::UnexpectedArguments,
            ExitClass::InvalidInvocation,
            "observe query received arguments outside the typed route contract",
            "HCT-OBSERVE query adapter",
            "invoke observe query with at most one typed --filter identifier",
            "read",
            "ultragoal --json observe query",
            "observability and dependent claims remain unchanged",
        ),
    )
}

fn stale_context() -> RuntimeOutcome {
    RuntimeOutcome::failure(
        ExitClass::ActionableFinding,
        Diagnostic::new(
            DiagnosticId::StaleContext,
            ExitClass::ActionableFinding,
            "the live candidate changed during the local observability query",
            "HCT-OBSERVE query adapter",
            "rebuild one LiveContext and query the separately bound current-candidate store",
            "read",
            "ultragoal --json observe query",
            "same-candidate observability and dependent claims are withheld",
        ),
    )
}

fn observability_unavailable() -> RuntimeOutcome {
    RuntimeOutcome::failure(
        ExitClass::UnsupportedCapability,
        Diagnostic::new(
            DiagnosticId::ObservabilityUnavailable,
            ExitClass::UnsupportedCapability,
            "the bounded local event store could not be opened or reconciled with this candidate",
            "HCT-OBSERVE local store",
            "repair the confined local store boundary or regenerate current-candidate events",
            "read",
            "ultragoal --json observe query",
            "observability and dependent claims remain withheld",
        ),
    )
}
