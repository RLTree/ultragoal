use crate::cli::successor::runtime::RuntimeOutcome;
use crate::cli::successor::{
    CheckProfile, OptionName, ParsedInvocation, ParsedValue, SuccessorCommand,
};
use crate::context::{BuildRequest, EffectClass, LiveContext};
use std::path::Path;
use std::thread;

mod check_projection;
mod failure_diagnostics;
mod namespace_law_adapter;
mod python_source_law_adapter;
mod zero_write_guard;

use check_projection::{CheckResult, LawFinding};

const SELF_LAW_CLAIM: &str = "cli-self-law-compliance";
const NAMESPACE_LAW_CLAIM: &str = "namespace-progressive-disclosure";
const LAW_JOBS: usize = 16;

pub(super) fn execute(root: &Path, invocation: &ParsedInvocation) -> RuntimeOutcome {
    let Some(claim_id) = claim_id(invocation) else {
        return super::failure(
            super::DiagnosticId::UnexpectedArguments,
            "strict checking requires exactly one typed claim identifier",
            "check strict invocation",
            "pass one --claim identifier from the adopted claim registry",
            "strict-check claims remain withheld",
        );
    };
    if !matches!(claim_id, SELF_LAW_CLAIM | NAMESPACE_LAW_CLAIM) {
        return failure_diagnostics::unsupported_claim();
    }
    let before = match zero_write_guard::capture(root) {
        Ok(snapshot) => snapshot,
        Err(_) => return failure_diagnostics::zero_write_snapshot_unavailable(),
    };
    let context = match strict_context(root) {
        Ok(context) => context,
        Err(()) => return super::context_unavailable(),
    };
    if context.effect().authorize(EffectClass::Read).is_err() {
        return super::failure(
            super::DiagnosticId::EffectMismatch,
            "strict checking lacks the declared read-only authority boundary",
            "check strict effect",
            "rerun through the canonical check strict route",
            "strict-check claims remain withheld",
        );
    }
    let (checks, mut findings) = match claim_id {
        SELF_LAW_CLAIM => run_law_checks(root, &context),
        NAMESPACE_LAW_CLAIM => namespace_law_adapter::run(root),
        _ => unreachable!("supported strict claims are checked before execution"),
    };
    if context.revalidate().is_err() {
        return failure_diagnostics::stale_candidate();
    }
    let after = match zero_write_guard::capture(root) {
        Ok(snapshot) => snapshot,
        Err(_) => return failure_diagnostics::zero_write_snapshot_unavailable(),
    };
    if before != after {
        return failure_diagnostics::hidden_write();
    }
    findings.sort();
    findings.dedup();
    check_projection::result(&context, claim_id, checks, findings)
}

fn strict_context(root: &Path) -> Result<LiveContext, ()> {
    LiveContext::build(
        BuildRequest::new(root)
            .with_effect(EffectClass::Read)
            .probe_tool("python3")
            .bind_non_secret_configuration(
                super::ADOPTED_HANDOFF_DIGEST_CONFIG_KEY,
                super::ADOPTED_HANDOFF_MANIFEST_SHA256,
            ),
    )
    .map_err(|_| ())
}

fn run_law_checks(root: &Path, context: &LiveContext) -> (Vec<CheckResult>, Vec<LawFinding>) {
    let python = context.capabilities().tool("python3").cloned();
    let (complete, python_source) = thread::scope(|scope| {
        let complete = scope.spawn(|| {
            crate::audit::cli::self_law::check(
                crate::audit::cli::self_law::CliSelfLawCheckRequest::new(root, LAW_JOBS),
            )
        });
        let python = scope.spawn(|| {
            python_source_law_adapter::PythonSourceLawRequest::bind(root, python.as_ref())
                .and_then(python_source_law_adapter::run)
        });
        (
            joined_complete(complete.join()),
            joined_python(python.join()),
        )
    });
    let checks = [
        ("complete-self-law", complete.len()),
        ("python-source-laws", python_source.len()),
    ]
    .into_iter()
    .map(|(check_id, finding_count)| CheckResult {
        check_id: check_id.to_owned(),
        status: if finding_count == 0 { "pass" } else { "fail" },
        finding_count,
    })
    .collect();
    let findings = complete.into_iter().chain(python_source).collect();
    (checks, findings)
}

fn joined_complete(
    result: thread::Result<
        Result<
            crate::audit::cli::self_law::CliSelfLawCheckResponse,
            crate::audit::cli::self_law::CliSelfLawCheckError,
        >,
    >,
) -> Vec<LawFinding> {
    match result {
        Ok(Ok(response)) => response
            .into_findings()
            .into_iter()
            .map(|finding| LawFinding {
                check_id: finding.check_id,
                detail: finding.detail,
            })
            .collect(),
        Ok(Err(error)) => vec![LawFinding {
            check_id: "complete-self-law".to_owned(),
            detail: format!("gate_error:{}", error.id()),
        }],
        Err(_) => vec![LawFinding {
            check_id: "complete-self-law".to_owned(),
            detail: "gate_worker_panicked".to_owned(),
        }],
    }
}

fn joined_python(
    result: thread::Result<
        Result<
            python_source_law_adapter::PythonSourceLawResponse,
            python_source_law_adapter::PythonSourceLawAdapterError,
        >,
    >,
) -> Vec<LawFinding> {
    let details = match result {
        Ok(Ok(response)) => response.into_findings(),
        Ok(Err(error)) => vec![format!("adapter_error:{}", error.id())],
        Err(_) => vec!["adapter_worker_panicked".to_owned()],
    };
    details
        .into_iter()
        .map(|detail| LawFinding {
            check_id: "python-source-laws".to_owned(),
            detail,
        })
        .collect()
}

fn claim_id(invocation: &ParsedInvocation) -> Option<&str> {
    if invocation.command != SuccessorCommand::Check(CheckProfile::Strict) {
        return None;
    }
    match invocation.arguments.as_slice() {
        [argument] if argument.name == OptionName::Claim => match &argument.value {
            ParsedValue::Identifier(value) => Some(value.as_str()),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests;
