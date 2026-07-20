use super::*;
use crate::cli::successor::command_contract::{OptionName, PackageAction, ParsedValue};
use std::path::Path;

#[cfg(test)]
use crate::cli::successor::command_contract::{OptionArgument, RelativePath};

pub(super) fn execute(_root: &Path, invocation: &ParsedInvocation) -> RuntimeOutcome {
    if !valid_invocation(invocation) {
        return invalid_invocation();
    }
    RuntimeOutcome::failure(
        ExitClass::UnsupportedCapability,
        Diagnostic::new(
            DiagnosticId::DownstreamToolUnavailable,
            ExitClass::UnsupportedCapability,
            DiagnosticDetails {
                cause: "no canonical runtime, discovery, and real-host adoption owner is available for package install-test",
                affected_surface: "HCT-DISTRIBUTION package install-test",
                repair: "complete the canonical installed-journey owner before retrying package installation",
                effect: "none",
                rerun: "ultragoal --json package install-test --input target/ultragoal/package.hugpkg --output target/ultragoal/install-test.json",
                ceiling: "no package install, cache, runtime, discovery, registry, readiness, or real-host claim is raised",
            },
        ),
    )
}

fn valid_invocation(invocation: &ParsedInvocation) -> bool {
    let ParsedInvocation {
        command: SuccessorCommand::Package(PackageAction::InstallTest),
        effect: EffectClass::WorkspaceWrite,
        arguments,
        ..
    } = invocation
    else {
        return false;
    };
    if arguments.len() != 2 {
        return false;
    }
    let input = arguments
        .iter()
        .find(|argument| argument.name == OptionName::Input);
    let output = arguments
        .iter()
        .find(|argument| argument.name == OptionName::Output);
    matches!(
        (input.map(|argument| &argument.value), output.map(|argument| &argument.value)),
        (Some(ParsedValue::RelativePath(_)), Some(ParsedValue::RelativePath(output)))
            if output_allowed(output.as_str())
    )
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
    RuntimeOutcome::failure(
        ExitClass::InvalidInvocation,
        Diagnostic::new(
            DiagnosticId::UnexpectedArguments,
            ExitClass::InvalidInvocation,
            DiagnosticDetails {
                cause: "package install-test requires bounded --input and disposable --output relative paths",
                affected_surface: "HCT-DISTRIBUTION package install-test",
                repair: "supply a package input and target/ultragoal/*.json output",
                effect: "none",
                rerun: "ultragoal --json package install-test --input target/ultragoal/package.hugpkg --output target/ultragoal/install-test.json",
                ceiling: "no package, install, cache, discovery, or runtime claim is available",
            },
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::successor::{EffectClass, OutputMode};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn valid_install_test_is_fail_closed_without_output_mutation() {
        let root = fixture_root();
        let output = root.join("target/ultragoal/install-test.json");
        fs::create_dir_all(output.parent().unwrap()).unwrap();
        fs::write(&output, b"preserve").unwrap();
        let invocation = ParsedInvocation {
            command: SuccessorCommand::Package(PackageAction::InstallTest),
            effect: EffectClass::WorkspaceWrite,
            output_mode: OutputMode::Json,
            arguments: vec![
                option(OptionName::Input, "target/ultragoal/package.hugpkg"),
                option(OptionName::Output, "target/ultragoal/install-test.json"),
            ],
        };
        let before = snapshot(&root);
        let outcome = execute(&root, &invocation);
        let after = snapshot(&root);
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(outcome.exit_class, ExitClass::UnsupportedCapability);
        assert_eq!(after, before);
    }

    fn option(name: OptionName, path: &str) -> OptionArgument {
        OptionArgument {
            name,
            value: ParsedValue::RelativePath(RelativePath(path.to_owned())),
        }
    }

    fn fixture_root() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "ultragoal-package-install-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        root
    }

    fn snapshot(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
        let mut entries = Vec::new();
        for entry in fs::read_dir(root).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                entries.extend(snapshot(&path));
            } else {
                let bytes = fs::read(&path).unwrap();
                entries.push((path, bytes));
            }
        }
        entries.sort_by(|left, right| left.0.cmp(&right.0));
        entries
    }
}
