use super::SELF_LAW_CLAIM;
use crate::cli::successor::ExitClass;
use crate::cli::successor::runtime::RuntimeOutcome;
use crate::context::{CandidateIdentity, LiveContext};
use serde::Serialize;

const ZERO_WRITE_SCOPE: &str = "recursive-active-worktree-plus-git-candidate-excluding-declared-build-and-frozen-review-caches";

#[derive(Serialize)]
struct StrictCheckResult<'a> {
    schema_version: &'static str,
    claim_id: &'a str,
    context_id: &'a str,
    candidate: &'a CandidateIdentity,
    effect: &'static str,
    zero_write_scope: &'static str,
    status: &'static str,
    checks: Vec<CheckResult>,
    findings: Vec<LawFinding>,
    supported_claims: Vec<&'static str>,
    unsupported_claims: Vec<&'static str>,
}

#[derive(Serialize)]
pub(super) struct CheckResult {
    pub(super) check_id: String,
    pub(super) status: &'static str,
    pub(super) finding_count: usize,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(super) struct LawFinding {
    pub(super) check_id: String,
    pub(super) detail: String,
}

pub(super) fn result(
    context: &LiveContext,
    claim_id: &str,
    checks: Vec<CheckResult>,
    findings: Vec<LawFinding>,
) -> RuntimeOutcome {
    let passed = findings.is_empty();
    let payload = StrictCheckResult {
        schema_version: "HarnessStrictCheck-v1",
        claim_id,
        context_id: context.context_id(),
        candidate: context.candidate(),
        effect: "read",
        zero_write_scope: ZERO_WRITE_SCOPE,
        status: if passed { "pass" } else { "fail" },
        checks,
        findings,
        supported_claims: if passed {
            vec![SELF_LAW_CLAIM]
        } else {
            Vec::new()
        },
        unsupported_claims: if passed {
            Vec::new()
        } else {
            vec![SELF_LAW_CLAIM]
        },
    };
    match serde_json::to_vec(&payload) {
        Ok(machine) if super::super::public_output_allowed(machine.len()) => {
            RuntimeOutcome::payload(
                if passed {
                    ExitClass::Success
                } else {
                    ExitClass::ActionableFinding
                },
                machine,
                format!("strict claim={claim_id} status={}", payload.status),
            )
        }
        _ => super::super::failure(
            super::super::DiagnosticId::ProjectionFailed,
            "the complete strict-check result could not be projected within the public bound",
            "check strict output",
            "repair the unbounded finding surface without dropping findings",
            "strict-check claims remain withheld",
        ),
    }
}
