//! Root-owned repository-fit public adapter surface.
//!
//! Read routes remain projection-only. Apply consumes one exact accepted plan,
//! persists restart authority in preprovisioned owner-only host state, and then
//! enters the sealed repository-fit production kernel.

use crate::cli::successor::runtime::{Diagnostic, DiagnosticId, RuntimeOutcome};
use crate::cli::successor::{
    EffectClass, ExitClass, FitAction, OptionName, ParsedInvocation, ParsedValue, SuccessorCommand,
};
use crate::context::LiveContext;
use crate::repository_fit::{
    AdapterErrorId, FitAdapterError, PreparedFitApply, inspect_target, plan_target,
    prepare_apply_request, verify_target,
};
use std::path::{Path, PathBuf};

const MAX_PLAN_RECORD_BYTES: u64 = 16 * 1024 * 1024;

pub(super) fn target_root(
    root: &Path,
    invocation: &ParsedInvocation,
) -> Result<PathBuf, RuntimeOutcome> {
    let mut target = None;
    for argument in &invocation.arguments {
        match (&argument.name, &argument.value) {
            (OptionName::Target, ParsedValue::RelativePath(path)) if target.is_none() => {
                target = Some(path.as_str())
            }
            (OptionName::Plan, ParsedValue::RelativePath(_))
            | (OptionName::AcceptPlan, ParsedValue::Identifier(_))
                if invocation.command == SuccessorCommand::Fit(FitAction::Apply) => {}
            _ => return Err(invalid_invocation()),
        }
    }
    Ok(target.map_or_else(|| root.to_path_buf(), |path| root.join(path)))
}

pub(super) fn inspect(context: &LiveContext, invocation: &ParsedInvocation) -> RuntimeOutcome {
    if !valid_read_invocation(invocation, FitAction::Inspect) {
        return invalid_invocation();
    }
    match inspect_target(context) {
        Ok(projection) => match projection.to_machine_bytes() {
            Ok(machine) => {
                let finding = !projection.compatible();
                RuntimeOutcome::payload(
                    if finding {
                        ExitClass::ActionableFinding
                    } else {
                        ExitClass::Success
                    },
                    machine,
                    format!(
                        "repository fit {} files={} claim_effect=none",
                        projection.classification(),
                        projection.file_count()
                    ),
                )
            }
            Err(failure) => adapter_failure(failure),
        },
        Err(failure) => adapter_failure(failure),
    }
}

pub(super) fn plan(context: &LiveContext, invocation: &ParsedInvocation) -> RuntimeOutcome {
    if !valid_read_invocation(invocation, FitAction::Plan) {
        return invalid_invocation();
    }
    match plan_target(context) {
        Ok(record) => match record.to_machine_bytes() {
            Ok(machine) => RuntimeOutcome::payload(
                if record.conflict_count() == 0 {
                    ExitClass::Success
                } else {
                    ExitClass::ActionableFinding
                },
                machine,
                format!(
                    "repository fit plan {} mutations={} conflicts={} claim_effect=none",
                    record.plan_sha256(),
                    record.mutation_count(),
                    record.conflict_count()
                ),
            ),
            Err(failure) => adapter_failure(failure),
        },
        Err(failure) => adapter_failure(failure),
    }
}

pub(super) fn verify(context: &LiveContext, invocation: &ParsedInvocation) -> RuntimeOutcome {
    if !valid_read_invocation(invocation, FitAction::Verify) {
        return invalid_invocation();
    }
    match verify_target(context) {
        Ok(projection) => match projection.to_machine_bytes() {
            Ok(machine) => RuntimeOutcome::payload(
                if projection.idempotent() {
                    ExitClass::Success
                } else {
                    ExitClass::ActionableFinding
                },
                machine,
                format!(
                    "repository fit verified={} matched_files={} claim_effect=none",
                    projection.idempotent(),
                    projection.matched_files()
                ),
            ),
            Err(failure) => adapter_failure(failure),
        },
        Err(failure) => adapter_failure(failure),
    }
}

pub(super) fn apply(
    context: &LiveContext,
    invocation: &ParsedInvocation,
    home: Option<&Path>,
) -> RuntimeOutcome {
    let arguments = match apply_arguments(invocation) {
        Ok(arguments) => arguments,
        Err(outcome) => return outcome,
    };
    let Some(home) = home else {
        return authority::host_state_unavailable();
    };
    match authority::recover_pending(context, home) {
        Ok(Some(outcome)) => return outcome,
        Ok(None) => {}
        Err(outcome) => return outcome,
    }
    let prepared = match prepare_apply_with_arguments(context, arguments) {
        Ok(prepared) => prepared,
        Err(outcome) => return outcome,
    };
    authority::execute(context, prepared, home)
}

struct ApplyArguments<'a> {
    plan_path: &'a str,
    accepted_plan: &'a str,
}

/// Reads one descriptor-anchored bounded plan record and returns an opaque
/// request. The shell-facing JSON renderer adds exactly one LF; accepting only
/// that framing in addition to canonical bytes keeps ordinary redirection
/// usable without accepting general whitespace or alternate encodings.
pub(super) fn prepare_apply(
    context: &LiveContext,
    invocation: &ParsedInvocation,
) -> Result<PreparedFitApply, RuntimeOutcome> {
    let arguments = apply_arguments(invocation)?;
    prepare_apply_with_arguments(context, arguments)
}

fn apply_arguments(invocation: &ParsedInvocation) -> Result<ApplyArguments<'_>, RuntimeOutcome> {
    if invocation.command != SuccessorCommand::Fit(FitAction::Apply)
        || invocation.effect != EffectClass::WorkspaceWrite
    {
        return Err(invalid_invocation());
    }
    let mut plan_path = None;
    let mut accepted_plan = None;
    for argument in &invocation.arguments {
        match (&argument.name, &argument.value) {
            (OptionName::Target, ParsedValue::RelativePath(_)) => {}
            (OptionName::Plan, ParsedValue::RelativePath(path)) if plan_path.is_none() => {
                plan_path = Some(path.as_str())
            }
            (OptionName::AcceptPlan, ParsedValue::Identifier(value)) if accepted_plan.is_none() => {
                accepted_plan = Some(value.as_str())
            }
            _ => return Err(invalid_invocation()),
        }
    }
    let (Some(plan_path), Some(accepted_plan)) = (plan_path, accepted_plan) else {
        return Err(invalid_invocation());
    };
    Ok(ApplyArguments {
        plan_path,
        accepted_plan,
    })
}

fn prepare_apply_with_arguments(
    context: &LiveContext,
    arguments: ApplyArguments<'_>,
) -> Result<PreparedFitApply, RuntimeOutcome> {
    let reads = context.begin_read_session().map_err(|_| stale_context())?;
    let absolute = reads.root().join(arguments.plan_path);
    let bytes = reads
        .read_bounded(&absolute, MAX_PLAN_RECORD_BYTES)
        .map_err(|_| invalid_plan())?;
    reads.revalidate().map_err(|_| stale_context())?;
    let canonical = if bytes.ends_with(b"\n") && !bytes[..bytes.len() - 1].ends_with(b"\n") {
        &bytes[..bytes.len() - 1]
    } else {
        bytes.as_slice()
    };
    prepare_apply_request(context, canonical, arguments.accepted_plan).map_err(adapter_failure)
}

fn valid_read_invocation(invocation: &ParsedInvocation, action: FitAction) -> bool {
    invocation.command == SuccessorCommand::Fit(action)
        && invocation.effect == EffectClass::Read
        && invocation.arguments.iter().all(|argument| {
            matches!(
                (&argument.name, &argument.value),
                (OptionName::Target, ParsedValue::RelativePath(_))
            )
        })
        && invocation
            .arguments
            .iter()
            .filter(|argument| argument.name == OptionName::Target)
            .count()
            <= 1
}

fn invalid_invocation() -> RuntimeOutcome {
    RuntimeOutcome::failure(
        ExitClass::InvalidInvocation,
        Diagnostic::new(
            DiagnosticId::UnexpectedArguments,
            ExitClass::InvalidInvocation,
            "the fit adapter received arguments outside the canonical typed route",
            "HCT-FIT public adapter",
            "reparse the exact fit command through the successor grammar",
            "read",
            "ultragoal --json fit inspect",
            "repository-fit and dependent claims remain unchanged",
        ),
    )
}

fn invalid_plan() -> RuntimeOutcome {
    RuntimeOutcome::failure(
        ExitClass::InvalidInvocation,
        Diagnostic::new(
            DiagnosticId::UnexpectedArguments,
            ExitClass::InvalidInvocation,
            "the plan path is not one bounded descriptor-anchored regular file",
            "HCT-FIT accepted plan",
            "write the exact canonical fit plan projection to a confined regular file",
            "read",
            "ultragoal --json fit plan",
            "no workspace effect is authorized or performed",
        ),
    )
}

fn stale_context() -> RuntimeOutcome {
    RuntimeOutcome::failure(
        ExitClass::ActionableFinding,
        Diagnostic::new(
            DiagnosticId::StaleContext,
            ExitClass::ActionableFinding,
            "the target candidate changed during fit plan observation",
            "HCT-FIT target binding",
            "rebuild one target LiveContext and recompute the fit plan",
            "read",
            "ultragoal --json fit plan",
            "no workspace effect is authorized or performed",
        ),
    )
}

fn adapter_failure(failure: FitAdapterError) -> RuntimeOutcome {
    let (class, diagnostic) = match failure.id() {
        AdapterErrorId::ContextStale | AdapterErrorId::StalePlan => {
            (ExitClass::ActionableFinding, DiagnosticId::StaleContext)
        }
        AdapterErrorId::AcceptanceMismatch | AdapterErrorId::PlanConflict => {
            (ExitClass::BlockedAuthority, DiagnosticId::AuthorityRequired)
        }
        AdapterErrorId::InvalidPlanRecord => (
            ExitClass::InvalidInvocation,
            DiagnosticId::UnexpectedArguments,
        ),
        AdapterErrorId::UnsupportedHost => (
            ExitClass::UnsupportedCapability,
            DiagnosticId::DownstreamToolUnavailable,
        ),
        AdapterErrorId::ApplyPermitMissing
        | AdapterErrorId::ApplyPermitInvalid
        | AdapterErrorId::ApplyPermitExpired
        | AdapterErrorId::ApplyPermitReplayed
        | AdapterErrorId::ApplyLeaseInvalid
        | AdapterErrorId::ApplyMutationScopeViolation
        | AdapterErrorId::ApplyOutcomeAmbiguous => {
            (ExitClass::BlockedAuthority, DiagnosticId::AuthorityRequired)
        }
        AdapterErrorId::InvalidTemplateCatalog
        | AdapterErrorId::TargetUnavailable
        | AdapterErrorId::ProjectionFailed
        | AdapterErrorId::EffectFailed
        | AdapterErrorId::ApplyOutcomeInvalid
        | AdapterErrorId::ApplyRolledBack => {
            (ExitClass::InternalFailure, DiagnosticId::ProjectionFailed)
        }
    };
    RuntimeOutcome::failure(
        class,
        Diagnostic::new(
            diagnostic,
            class,
            failure.cause(),
            "HCT-FIT candidate adapter",
            "rebuild the current target context and exact canonical template-bound fit plan",
            "read_or_unexecuted_workspace_request",
            "ultragoal --json fit plan",
            "no live-repository, readiness, release, or completion claim is raised",
        ),
    )
}

mod authority;
