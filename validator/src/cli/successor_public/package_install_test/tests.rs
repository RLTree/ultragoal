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
        ],
    };
    assert_eq!(
        invocation_paths(&invocation),
        Some((
            "target/ultragoal/package.hugpkg",
            "target/ultragoal/install-test.json"
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

fn option(name: OptionName, path: &str) -> OptionArgument {
    OptionArgument {
        name,
        value: ParsedValue::RelativePath(RelativePath(path.to_owned())),
    }
}
