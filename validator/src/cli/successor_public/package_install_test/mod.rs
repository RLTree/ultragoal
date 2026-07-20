use super::*;
use crate::cli::successor::command_contract::{OptionName, PackageAction, ParsedValue};
use crate::plugin_product::host_lifecycle::{InstallTestError, execute as run_install_test};
use std::path::Path;

pub(super) fn dispatch(root: &Path, invocation: &ParsedInvocation) -> RuntimeOutcome {
    let context = match super::workspace_context(root) {
        Ok(context) => context,
        Err(()) => return super::context_unavailable(),
    };
    execute(&context, invocation)
}

pub(super) fn execute(context: &LiveContext, invocation: &ParsedInvocation) -> RuntimeOutcome {
    let Some((input, output)) = paths(invocation) else {
        return invalid_invocation();
    };
    match run_install_test(context, input, output) {
        Ok(report) => match serde_json::to_vec(&report) {
            Ok(machine) if public_output_allowed(machine.len()) => RuntimeOutcome::payload(
                ExitClass::Success,
                machine,
                "isolated package install and cache observations completed".to_owned(),
            ),
            _ => output_failure(),
        },
        Err(error) => failure(error),
    }
}

fn paths(invocation: &ParsedInvocation) -> Option<(&str, &str)> {
    let ParsedInvocation {
        command: SuccessorCommand::Package(PackageAction::InstallTest),
        effect: EffectClass::WorkspaceWrite,
        arguments,
        ..
    } = invocation
    else {
        return None;
    };
    if arguments.len() != 2 {
        return None;
    }
    let input = arguments
        .iter()
        .find(|argument| argument.name == OptionName::Input)?;
    let output = arguments
        .iter()
        .find(|argument| argument.name == OptionName::Output)?;
    match (&input.value, &output.value) {
        (ParsedValue::RelativePath(input), ParsedValue::RelativePath(output))
            if output_allowed(output.as_str()) =>
        {
            Some((input.as_str(), output.as_str()))
        }
        _ => None,
    }
}

fn output_allowed(path: &str) -> bool {
    let Some(name) = path.strip_prefix("target/ultragoal/") else {
        return false;
    };
    let Some(stem) = name.strip_suffix(".json") else {
        return false;
    };
    !stem.is_empty()
        && !name.contains('/')
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn invalid_invocation() -> RuntimeOutcome {
    diagnostic(
        DiagnosticId::UnexpectedArguments,
        ExitClass::InvalidInvocation,
        "package install-test requires bounded --input and disposable --output relative paths",
        "supply a current package archive and target/ultragoal/*.json output",
        "package, install, cache, discovery, and runtime claims remain withheld",
    )
}

fn failure(error: InstallTestError) -> RuntimeOutcome {
    let (cause, repair) = match error {
        InstallTestError::Context => (
            "the workspace context is no longer current",
            "refresh the workspace and retry",
        ),
        InstallTestError::Input => (
            "the requested package input is unavailable or unsafe",
            "use one confined current package input",
        ),
        InstallTestError::Package => (
            "the input does not match the current source-bound package",
            "rebuild the current package and retry",
        ),
        InstallTestError::Custody => (
            "isolated host custody could not settle safely",
            "inspect the retained isolated recovery state before retry",
        ),
        InstallTestError::Effect => (
            "isolated install or cache observation did not match",
            "repair the isolated package or host effect and retry",
        ),
        InstallTestError::Output => (
            "the local verification result could not be atomically written",
            "use a fresh disposable output path",
        ),
    };
    diagnostic(
        DiagnosticId::DownstreamToolUnavailable,
        ExitClass::UnsupportedCapability,
        cause,
        repair,
        "only isolated package, install, and cache observations may be available; marketplace, app-registry, discovery, runtime, and real-host claims remain withheld",
    )
}

fn output_failure() -> RuntimeOutcome {
    failure(InstallTestError::Output)
}

fn diagnostic(
    id: DiagnosticId,
    exit: ExitClass,
    cause: &'static str,
    repair: &'static str,
    ceiling: &'static str,
) -> RuntimeOutcome {
    RuntimeOutcome::failure(
        exit,
        Diagnostic::new(
            id,
            exit,
            DiagnosticDetails {
                cause,
                affected_surface: "HCT-DISTRIBUTION package install-test",
                repair,
                effect: "workspace_write",
                rerun: "ultragoal --json package install-test --input target/ultragoal/package.hugpkg --output target/ultragoal/install-test.json",
                ceiling,
            },
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::output_allowed;

    #[test]
    fn install_test_output_stays_in_disposable_product_namespace() {
        assert!(output_allowed("target/ultragoal/install-test.json"));
        assert!(!output_allowed("target/ultragoal/nested/install-test.json"));
        assert!(!output_allowed("plugin-manifest.json"));
    }
}
