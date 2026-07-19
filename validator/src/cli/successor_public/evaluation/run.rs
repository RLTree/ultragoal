use super::*;

pub(crate) fn execute(invocation: &ParsedInvocation) -> RuntimeOutcome {
    if !valid_run_invocation(invocation) {
        return invalid_run_invocation();
    }
    terminal_execution_unsupported()
}

fn valid_run_invocation(invocation: &ParsedInvocation) -> bool {
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
                    (ParsedValue::RelativePath(_), ParsedValue::RelativePath(_)) => true,
                    _ => false,
                }
            }
            _ => false,
        },
        _ => false,
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
