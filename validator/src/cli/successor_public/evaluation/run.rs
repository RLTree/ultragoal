use super::*;
use crate::cli::capture::ScheduledFixtureEvaluationBridge;
use crate::evaluation::EvaluationRunAdmission;

pub(crate) fn execute(context: &LiveContext, invocation: &ParsedInvocation) -> RuntimeOutcome {
    let Some((spec_path, output_path)) = run_paths(invocation) else {
        return invalid_run_invocation();
    };
    if !ScheduledFixtureEvaluationBridge::terminal_execution_supported() {
        return terminal_execution_unsupported();
    }
    let Ok(candidate_id) = candidate_id(context) else {
        return projection_failed();
    };
    let Ok(spec) = specification::load(context, spec_path, &candidate_id) else {
        return invalid_specification();
    };
    let Ok(admission) = EvaluationRunAdmission::issue(context, &spec, output_path) else {
        return stale_context();
    };
    let mut bridge = ScheduledFixtureEvaluationBridge::new(context.worktree_root());
    let Ok(run) = admission.execute(&mut bridge) else {
        return execution_failed();
    };
    let bytes = run.public_result();
    if !public_output_allowed(bytes.len()) {
        return projection_failed();
    }
    RuntimeOutcome::payload(
        ExitClass::Success,
        bytes.to_vec(),
        "evaluation run completed with an authenticated terminal result".to_owned(),
    )
}

fn run_paths(invocation: &ParsedInvocation) -> Option<(&str, &str)> {
    match invocation {
        ParsedInvocation {
            command: SuccessorCommand::Eval(EvalAction::Run),
            effect: EffectClass::WorkspaceWrite,
            arguments,
            ..
        } => match arguments.as_slice() {
            [spec, output]
                if spec.name == OptionName::Spec && output.name == OptionName::Output =>
            {
                match (&spec.value, &output.value) {
                    (ParsedValue::RelativePath(spec), ParsedValue::RelativePath(output)) => {
                        Some((spec.as_str(), output.as_str()))
                    }
                    _ => None,
                }
            }
            _ => None,
        },
        _ => None,
    }
}

fn invalid_run_invocation() -> RuntimeOutcome {
    failure(
        DiagnosticId::UnexpectedArguments,
        ExitClass::InvalidInvocation,
        "the evaluation run requires bounded --spec and --output relative paths",
        "HCT-EVAL public execution admission",
        "invoke eval run through the canonical successor grammar",
    )
}

fn execution_failed() -> RuntimeOutcome {
    failure(
        DiagnosticId::ProjectionFailed,
        ExitClass::ActionableFinding,
        "the evaluation run could not obtain a terminal authenticated result",
        "HCT-EVAL execution custody",
        "repair the named fixture, recovery state, or bound evaluation inputs before retrying",
    )
}

fn terminal_execution_unsupported() -> RuntimeOutcome {
    RuntimeOutcome::failure(
        ExitClass::UnsupportedCapability,
        Diagnostic::new(
            DiagnosticId::DownstreamToolUnavailable,
            ExitClass::UnsupportedCapability,
            DiagnosticDetails {
                cause: "the host cannot jointly confine a fixture child and prove identity-conditioned cleanup; evaluation custody remains recovery-required",
                affected_surface: "HCT-EVAL terminal execution",
                repair: "use a host with both required capabilities or retain recovery custody without retrying the fixture",
                effect: "none",
                rerun: "ultragoal --json eval run --spec evaluation/run.json --output results/run.json",
                ceiling: "no evaluation result, receipt, runtime, readiness, release, or completion claim is raised",
            },
        ),
    )
}
