//! Read-only public admission for bounded evaluation-specification audits.

use super::*;
use crate::cli::successor::command_contract::EvalAction;
use crate::cli::successor::runtime::{Diagnostic, DiagnosticDetails, DiagnosticId};
use crate::cli::successor::{
    EffectClass, ExitClass, OptionName, ParsedInvocation, ParsedValue, SuccessorCommand,
};
use crate::context::LiveContext;
use sha2::{Digest, Sha256};

mod run;
mod specification;

pub(super) use run::execute as run;

pub(super) fn audit(context: &LiveContext, invocation: &ParsedInvocation) -> RuntimeOutcome {
    let Some(spec_path) = spec_path(invocation) else {
        return invalid_invocation();
    };
    let Ok(candidate_id) = candidate_id(context) else {
        return projection_failed();
    };
    let Ok(spec) = specification::load(context, spec_path, &candidate_id) else {
        return invalid_specification();
    };
    let audit = spec.audit(context.context_id(), &candidate_id);
    if context.revalidate().is_err() {
        return stale_context();
    }
    let machine = serde_json::to_vec(&serde_json::json!({
        "schema_version": "EvaluationAudit-v1",
        "context_id": context.context_id(),
        "candidate_id": candidate_id,
        "audit": audit,
        "claim_effect": "none",
    }));
    match machine {
        Ok(machine) if public_output_allowed(machine.len()) => RuntimeOutcome::payload(
            if audit.eligible() {
                ExitClass::Success
            } else {
                ExitClass::ActionableFinding
            },
            machine,
            format!(
                "evaluation audit eligible={} findings={}",
                audit.eligible(),
                audit.findings().len()
            ),
        ),
        _ => projection_failed(),
    }
}

fn spec_path(invocation: &ParsedInvocation) -> Option<&str> {
    match invocation {
        ParsedInvocation {
            command: SuccessorCommand::Eval(EvalAction::Audit),
            effect: EffectClass::Read,
            arguments,
            ..
        } => match arguments.as_slice() {
            [argument] if argument.name == OptionName::Spec => match &argument.value {
                ParsedValue::RelativePath(path) => Some(path.as_str()),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

fn candidate_id(context: &LiveContext) -> Result<String, ()> {
    serde_json::to_vec(context.candidate())
        .map(|bytes| format!("sha256:{:x}", Sha256::digest(bytes)))
        .map_err(|_| ())
}

fn invalid_invocation() -> RuntimeOutcome {
    failure(
        DiagnosticId::UnexpectedArguments,
        ExitClass::InvalidInvocation,
        "the evaluation audit requires exactly one bounded --spec relative path",
        "HCT-EVAL public audit admission",
        "invoke eval audit through the canonical successor grammar",
    )
}

fn invalid_specification() -> RuntimeOutcome {
    failure(
        DiagnosticId::UnexpectedArguments,
        ExitClass::InvalidInvocation,
        "the evaluation specification is not one bounded canonical audit document",
        "HCT-EVAL public audit admission",
        "supply one current, typed evaluation audit specification under the repository root",
    )
}

fn stale_context() -> RuntimeOutcome {
    failure(
        DiagnosticId::StaleContext,
        ExitClass::ActionableFinding,
        "the candidate changed during evaluation audit admission",
        "HCT-EVAL candidate binding",
        "rebuild the live context and rerun eval audit against the current candidate",
    )
}

fn projection_failed() -> RuntimeOutcome {
    failure(
        DiagnosticId::ProjectionFailed,
        ExitClass::InternalFailure,
        "the bounded evaluation audit projection could not be produced",
        "HCT-EVAL public output",
        "repair the evaluation audit projection without exposing evaluation custody",
    )
}

fn failure(
    id: DiagnosticId,
    class: ExitClass,
    cause: &'static str,
    surface: &'static str,
    repair: &'static str,
) -> RuntimeOutcome {
    RuntimeOutcome::failure(
        class,
        Diagnostic::new(
            id,
            class,
            DiagnosticDetails {
                cause,
                affected_surface: surface,
                repair,
                effect: "read",
                rerun: "ultragoal --json eval audit --spec evaluation-audit.json",
                ceiling: "evaluation execution, promotion, runtime, readiness, and completion claims remain withheld",
            },
        ),
    )
}
