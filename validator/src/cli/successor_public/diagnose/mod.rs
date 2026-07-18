use super::public_output_allowed;
use crate::cli::successor::runtime::{Diagnostic, DiagnosticDetails, DiagnosticId, RuntimeOutcome};
use crate::cli::successor::{
    EffectClass, ExitClass, OptionName, ParsedInvocation, ParsedValue, SuccessorCommand,
};
use crate::context::LiveContext;
use crate::state::{Finding, ProductState};
use serde_json::Value;
use std::path::Path;

mod causal;

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
        Some(finding) => causal::evaluate(root, context, finding),
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
