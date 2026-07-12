use super::local_store::LocalStore;
use super::public_output_allowed;
use crate::cli::successor::runtime::{Diagnostic, DiagnosticId, RuntimeOutcome};
use crate::cli::successor::{
    EffectClass, ExitClass, OptionName, ParsedInvocation, ParsedValue, SuccessorCommand,
};
use crate::context::LiveContext;
use crate::observability::{EventQuery, EventStore};
use crate::state::{Finding, ProductState};
use serde_json::{Value, json};
use std::path::Path;

const SOURCE_ID: &str = "successor-runtime";
const MISSING_EVENT_ID: &str = "diagnose-missing-event";

pub(super) fn diagnose_local(
    root: &Path,
    context: &LiveContext,
    state: &ProductState,
    invocation: &ParsedInvocation,
) -> RuntimeOutcome {
    if invocation.command != SuccessorCommand::Diagnose || invocation.effect != EffectClass::Read {
        return invalid_invocation();
    }
    let requested = match invocation.arguments.as_slice() {
        [] => None,
        [argument]
            if argument.name == OptionName::Finding
                && matches!(argument.value, ParsedValue::Identifier(_)) =>
        {
            let ParsedValue::Identifier(value) = &argument.value else {
                unreachable!()
            };
            Some(value.as_str())
        }
        _ => return invalid_invocation(),
    };
    let selected = match requested {
        Some(identifier) => match select_finding(state, identifier) {
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
        Some(finding) => causal_status(root, context, finding),
        None => not_evaluated(),
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

fn causal_status(root: &Path, context: &LiveContext, finding: &Finding) -> Value {
    let store = match LocalStore::open(root, context, SOURCE_ID) {
        Ok(store) => store,
        Err(()) => return unavailable(),
    };
    let query = match EventQuery::for_context(context, SOURCE_ID) {
        Ok(query) => query,
        Err(_) => return unavailable(),
    };
    if store.status() == "absent" {
        return with_policy("absent", missing_store(), None);
    }
    let events = store.query_diagnostic(&query);
    let target = events.as_ref().ok().and_then(|events| {
        events
            .iter()
            .rev()
            .find(|event| {
                event.finding_refs().contains(&finding.finding_id)
                    || event.repair_refs().contains(&finding.repair.repair_id)
            })
            .map(|event| event.event_id().to_owned())
    });
    let explanation = match store.explain(&query, target.as_deref().unwrap_or(MISSING_EVENT_ID)) {
        Ok(Some(explanation)) => match serde_json::to_value(explanation) {
            Ok(value) => value,
            Err(_) => return unavailable(),
        },
        _ => return unavailable(),
    };
    if !store.revalidate() || context.revalidate().is_err() {
        return unavailable();
    }
    with_policy("available", explanation, target)
}

fn with_policy(store_status: &str, explanation: Value, matched_event_id: Option<String>) -> Value {
    json!({
        "store_status": store_status,
        "matched_event_id": matched_event_id,
        "explanation": explanation,
        "policy": local_policy(),
        "claim_effect": "none"
    })
}

fn not_evaluated() -> Value {
    json!({
        "store_status": "not_opened",
        "matched_event_id": null,
        "explanation": {
            "schema_version": "CausalExplanation-v1",
            "classification": "not-evaluated",
            "summary": "Select one current finding to correlate it with candidate-bound local events.",
            "causal_event_ids": [],
            "diagnostic_code": "observe-cause-not-evaluated:finding-required",
            "repair": "Run diagnose with one current --finding identifier.",
            "claim_effect": "none"
        },
        "policy": local_policy(),
        "claim_effect": "none"
    })
}

fn missing_store() -> Value {
    json!({
        "schema_version": "CausalExplanation-v1",
        "classification": "missing-evidence",
        "summary": "No local event store exists for this candidate; causality is withheld.",
        "causal_event_ids": [],
        "diagnostic_code": "observe-evidence-missing:store-empty",
        "repair": "Run the affected operation with bounded local event emission, then diagnose the current finding again.",
        "claim_effect": "none"
    })
}

fn unavailable() -> Value {
    with_policy(
        "unavailable",
        json!({
            "schema_version": "CausalExplanation-v1",
            "classification": "unavailable",
            "summary": "The local event boundary could not be safely reconciled; causality is withheld.",
            "causal_event_ids": [],
            "diagnostic_code": "observe-cause-unavailable:store-boundary",
            "repair": "Preserve the store, repair its confined identity or integrity, and rerun diagnosis.",
            "claim_effect": "none"
        }),
        None,
    )
}

fn local_policy() -> Value {
    json!({
        "schema_version": "LocalObservabilityPolicy-v1",
        "store_limit_bytes": EventStore::supported_store_limit_bytes(),
        "event_limit": EventStore::supported_event_limit(),
        "scan_row_limit": EventStore::supported_scan_limit(),
        "query_result_limit": EventStore::supported_result_limit(),
        "deletion": "explicit-clear-api",
        "external_export": "disabled-safe-default-OD-004-OD-007"
    })
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
            cause,
            "HCT-STATE + HCT-OBSERVE diagnosis",
            repair,
            "read",
            "ultragoal --json diagnose",
            ceiling,
        ),
    )
}
