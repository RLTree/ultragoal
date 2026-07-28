use super::*;
use crate::cli::successor::command_contract::{OptionArgument, RelativePath};
use crate::cli::successor::{EffectClass, OutputMode};

#[test]
fn exact_install_test_paths_are_accepted() {
    let invocation = ParsedInvocation {
        command: SuccessorCommand::Package(PackageAction::InstallTest),
        effect: EffectClass::WorkspaceWrite,
        output_mode: OutputMode::Json,
        arguments: vec![
            option(OptionName::Input, "target/ultragoal/package.hugpkg"),
            option(OptionName::Output, "target/ultragoal/install-test.json"),
            option(OptionName::Cli, "target/ultragoal/release/ultragoal"),
        ],
    };
    assert_eq!(
        invocation_paths(&invocation),
        Some((
            "target/ultragoal/package.hugpkg",
            "target/ultragoal/install-test.json",
            "target/ultragoal/release/ultragoal",
            false,
        ))
    );
}

#[test]
fn explicit_retention_is_the_only_optional_install_test_argument() {
    let invocation = ParsedInvocation {
        command: SuccessorCommand::Package(PackageAction::InstallTest),
        effect: EffectClass::WorkspaceWrite,
        output_mode: OutputMode::Json,
        arguments: vec![
            option(OptionName::Input, "target/ultragoal/package.hugpkg"),
            option(OptionName::Output, "target/ultragoal/install-test.json"),
            option(OptionName::Cli, "target/ultragoal/release/ultragoal"),
            OptionArgument {
                name: OptionName::RetainIsolatedRoot,
                value: ParsedValue::Flag,
            },
        ],
    };
    assert_eq!(
        invocation_paths(&invocation),
        Some((
            "target/ultragoal/package.hugpkg",
            "target/ultragoal/install-test.json",
            "target/ultragoal/release/ultragoal",
            true,
        ))
    );
}

#[test]
fn output_must_remain_a_single_bounded_json_leaf() {
    assert!(output_allowed("target/ultragoal/install-test.json"));
    assert!(!output_allowed("install-test.json"));
    assert!(!output_allowed("target/ultragoal/nested/install-test.json"));
    assert!(!output_allowed("target/ultragoal/install-test.txt"));
}

#[test]
fn package_archive_input_stays_in_the_disposable_package_namespace() {
    assert!(
        super::super::package_dispatch::package_archive_input_allowed(
            "target/ultragoal/package.hugpkg"
        )
    );
    for path in [
        "package.hugpkg",
        "target/ultragoal/nested/package.hugpkg",
        "target/ultragoal/package.zip",
        "target/ultragoal/.hugpkg",
    ] {
        assert!(
            !super::super::package_dispatch::package_archive_input_allowed(path),
            "{path}"
        );
    }
}

fn option(name: OptionName, path: &str) -> OptionArgument {
    OptionArgument {
        name,
        value: ParsedValue::RelativePath(RelativePath(path.to_owned())),
    }
}
