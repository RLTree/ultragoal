use super::*;
#[cfg(unix)]
use crate::plugin_product::agent_discovery::{reset_test_io_counts, test_io_counts};
#[cfg(unix)]
use std::path::PathBuf;

#[cfg(unix)]
#[test]
pub(crate) fn inspect_capabilities_withholds_relative_home_without_authority_io() {
    let package = Repository::new("relative-home-package");
    let project = Repository::new("relative-home-project");
    let home = project.root.with_extension("relative-home");
    package.install_agent_authority(&home);
    let before_package = tree(&package.root);
    let before_project = tree(&project.root);
    let before_home = tree(&home);

    reset_test_io_counts();
    let relative_home = PathBuf::from("relative-home");
    let streams = execute_invocation_with_home(
        &project.root,
        invocation(&package.root),
        Some(&relative_home),
    )
    .render(OutputMode::Json);

    assert_eq!(streams.exit_code, 0);
    let value: serde_json::Value = serde_json::from_slice(&streams.stdout).unwrap();
    assert_eq!(value["agent_authority"]["status"], "unavailable");
    assert_eq!(
        value["agent_authority"]["observation_code"],
        "home-unavailable"
    );
    assert_eq!(test_io_counts(), Default::default());
    assert_eq!(tree(&package.root), before_package);
    assert_eq!(tree(&project.root), before_project);
    assert_eq!(tree(&home), before_home);
    fs::remove_dir_all(home).unwrap();
}

#[cfg(unix)]
fn invocation(package_root: &Path) -> ParsedInvocation {
    let ParseOutcome::Invocation(invocation) = parse_args([
        "--json",
        "inspect",
        "capabilities",
        "--package-root",
        package_root.to_str().unwrap(),
    ])
    .unwrap() else {
        panic!("expected capability invocation")
    };
    invocation
}
