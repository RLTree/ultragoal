use super::*;
use crate::cli::successor::command_contract::HelpTarget;
use crate::cli::successor::{
    InspectTarget, OptionName, ParseErrorId, ParsedInvocation, ParsedValue, SuccessorCommand,
    render_help,
};

#[test]
pub(crate) fn inspect_capabilities_verifies_distinct_authority_roots_without_writes() {
    let package = Repository::new("capabilities-package");
    let project = Repository::new("capabilities-project");
    let home = project.root.with_extension("capabilities-home");
    package.install_agent_authority(&home);
    let before_package = tree(&package.root);
    let before_project = tree(&project.root);
    let before_home = tree(&home);
    let package_status = package.status();
    let project_status = project.status();

    let streams =
        execute_invocation_with_home(&project.root, invocation(Some(&package.root)), Some(&home))
            .render(OutputMode::Json);

    assert_eq!(streams.exit_code, 0);
    assert!(streams.stderr.is_empty());
    let output = String::from_utf8(streams.stdout).unwrap();
    let value: serde_json::Value = serde_json::from_str(&output).unwrap();
    let authority = &value["agent_authority"];
    assert_eq!(value["schema_version"], "HarnessCapabilities-v1");
    assert_eq!(authority["status"], "verified");
    assert_eq!(authority["observation_code"], "verified");
    assert_eq!(authority["claim_effect"], false);
    assert_eq!(authority["host_discovery"], "unavailable");
    assert_eq!(authority["runtime_exposure"], "unavailable");
    assert_ne!(
        authority["binding"]["source_root_id"],
        authority["binding"]["project_root_id"]
    );
    assert!(authority["roles"].as_array().unwrap().iter().all(|role| {
        role["match_state"] == "verified"
            && role["package"] == true
            && role["installed"] == true
            && role["cache"] == true
            && role["global"] == false
            && role["project"] == true
    }));
    for root in [&package.root, &project.root, &home] {
        assert!(!output.contains(root.to_str().unwrap()));
    }
    assert_eq!(tree(&package.root), before_package);
    assert_eq!(package.status(), package_status);
    assert_eq!(tree(&project.root), before_project);
    assert_eq!(project.status(), project_status);
    assert_eq!(tree(&home), before_home);
    fs::remove_dir_all(home).unwrap();
}

#[test]
pub(crate) fn inspect_capabilities_blocks_aliased_authority_roots_without_writes() {
    let repo = Repository::new("capabilities-agent-authority");
    let home = repo.root.with_extension("capabilities-home");
    repo.install_agent_authority(&home);
    let before_tree = tree(&repo.root);
    let before_status = repo.status();
    let before_home_tree = tree(&home);
    let alias = repo.root.join(".");

    let streams = execute_invocation_with_home(&repo.root, invocation(Some(&alias)), Some(&home))
        .render(OutputMode::Json);

    assert_eq!(streams.exit_code, 1);
    assert!(streams.stderr.is_empty());
    let output = String::from_utf8(streams.stdout).unwrap();
    let value: serde_json::Value = serde_json::from_str(&output).unwrap();
    let authority = &value["agent_authority"];
    assert_eq!(authority["status"], "blocked");
    assert_eq!(authority["observation_code"], "identity-mismatch");
    assert_eq!(authority["claim_effect"], false);
    assert!(
        authority["roles"]
            .as_array()
            .unwrap()
            .iter()
            .all(|role| role["match_state"] == "blocked")
    );
    for root in [&repo.root, &home] {
        assert!(!output.contains(root.to_str().unwrap()));
    }
    assert_eq!(tree(&repo.root), before_tree);
    assert_eq!(repo.status(), before_status);
    assert_eq!(tree(&home), before_home_tree);
    fs::remove_dir_all(home).unwrap();
}

#[test]
pub(crate) fn inspect_capabilities_withholds_authority_without_package_root() {
    let repo = Repository::new("capabilities-no-package-root");
    let home = repo.root.with_extension("capabilities-home");
    repo.install_agent_authority(&home);
    let before_tree = tree(&repo.root);
    let before_home = tree(&home);

    let streams = execute_invocation_with_home(&repo.root, invocation(None), Some(&home))
        .render(OutputMode::Json);

    assert_eq!(streams.exit_code, 0);
    let value: serde_json::Value = serde_json::from_slice(&streams.stdout).unwrap();
    let authority = &value["agent_authority"];
    assert_eq!(authority["status"], "unavailable");
    assert_eq!(authority["observation_code"], "package-root-unavailable");
    assert!(
        authority["roles"]
            .as_array()
            .unwrap()
            .iter()
            .all(|role| role["match_state"] == "unavailable")
    );
    assert_eq!(tree(&repo.root), before_tree);
    assert_eq!(tree(&home), before_home);
    fs::remove_dir_all(home).unwrap();
}

#[test]
pub(crate) fn inspect_capabilities_withholds_host_authority_when_home_is_absent() {
    let repo = Repository::new("capabilities-no-home");
    let before_tree = tree(&repo.root);
    let before_status = repo.status();

    let streams = execute_invocation_with_home(&repo.root, invocation(Some(&repo.root)), None)
        .render(OutputMode::Json);

    assert_eq!(streams.exit_code, 0);
    let value: serde_json::Value = serde_json::from_slice(&streams.stdout).unwrap();
    let authority = &value["agent_authority"];
    assert_eq!(authority["status"], "unavailable");
    assert_eq!(authority["observation_code"], "home-unavailable");
    assert!(
        authority["roles"]
            .as_array()
            .unwrap()
            .iter()
            .all(|role| role["match_state"] == "unavailable")
    );
    assert_eq!(tree(&repo.root), before_tree);
    assert_eq!(repo.status(), before_status);
}

#[test]
pub(crate) fn inspect_capabilities_catalog_exposes_a_typed_package_root() {
    let invocation = invocation(Some(Path::new("/package")));
    assert!(matches!(
        invocation.arguments.as_slice(),
        [argument]
            if argument.name == OptionName::PackageRoot
                && matches!(&argument.value, ParsedValue::HostPath(_))
    ));
    let error = parse_args([
        "--json",
        "inspect",
        "capabilities",
        "--package-root",
        "relative-package",
    ])
    .unwrap_err();
    assert_eq!(error.error.id(), ParseErrorId::InvalidPath);
    let help = render_help(
        HelpTarget::Command(SuccessorCommand::Inspect(InspectTarget::Capabilities)),
        OutputMode::Json,
    );
    assert!(help.contains("--package-root"));
    assert!(help.contains("host-path"));
}

fn invocation(package_root: Option<&Path>) -> ParsedInvocation {
    let mut args = vec![
        "--json".to_owned(),
        "inspect".to_owned(),
        "capabilities".to_owned(),
    ];
    if let Some(package_root) = package_root {
        args.push("--package-root".to_owned());
        args.push(package_root.to_string_lossy().into_owned());
    }
    let ParseOutcome::Invocation(invocation) = parse_args(args).unwrap() else {
        panic!("expected capability invocation")
    };
    invocation
}
