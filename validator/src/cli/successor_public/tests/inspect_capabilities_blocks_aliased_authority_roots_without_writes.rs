use super::*;

#[test]
pub(crate) fn inspect_capabilities_blocks_aliased_authority_roots_without_writes() {
    let repo = Repository::new("capabilities-agent-authority");
    let home = repo.root.with_extension("capabilities-home");
    repo.install_agent_authority(&home);
    let before_tree = tree(&repo.root);
    let before_status = repo.status();
    let before_home_tree = tree(&home);
    let ParseOutcome::Invocation(invocation) =
        parse_args(["--json", "inspect", "capabilities"]).unwrap()
    else {
        panic!("expected capability invocation")
    };

    let streams =
        execute_invocation_with_home(&repo.root, invocation, Some(&home)).render(OutputMode::Json);
    assert_eq!(
        streams.exit_code,
        1,
        "{}",
        String::from_utf8_lossy(&streams.stderr)
    );
    assert!(streams.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&streams.stdout).unwrap();
    let authority = &value["agent_authority"];
    assert_eq!(value["schema_version"], "HarnessCapabilities-v1");
    assert_eq!(authority["status"], "blocked");
    assert_eq!(authority["observation_code"], "identity-mismatch");
    assert_eq!(authority["claim_effect"], false);
    assert_eq!(authority["host_discovery"], "unavailable");
    assert_eq!(authority["runtime_exposure"], "unavailable");
    assert_eq!(authority["roles"].as_array().unwrap().len(), 6);
    assert!(
        authority["roles"]
            .as_array()
            .unwrap()
            .iter()
            .all(|role| role["match_state"] == "blocked")
    );
    let output = String::from_utf8(streams.stdout).unwrap();
    assert!(!output.contains(repo.root.to_str().unwrap()));
    assert!(!output.contains(home.to_str().unwrap()));
    assert_eq!(tree(&repo.root), before_tree);
    assert_eq!(repo.status(), before_status);
    assert_eq!(tree(&home), before_home_tree);
    fs::remove_dir_all(home).unwrap();
}

#[test]
pub(crate) fn inspect_capabilities_withholds_host_authority_when_home_is_absent() {
    let repo = Repository::new("capabilities-no-home");
    let before_tree = tree(&repo.root);
    let before_status = repo.status();
    let ParseOutcome::Invocation(invocation) =
        parse_args(["--json", "inspect", "capabilities"]).unwrap()
    else {
        panic!("expected capability invocation")
    };

    let streams =
        execute_invocation_with_home(&repo.root, invocation, None).render(OutputMode::Json);
    assert_eq!(streams.exit_code, 0);
    let value: serde_json::Value = serde_json::from_slice(&streams.stdout).unwrap();
    let authority = &value["agent_authority"];
    assert_eq!(authority["status"], "unavailable");
    assert_eq!(authority["claim_effect"], false);
    assert_eq!(authority["roles"].as_array().unwrap().len(), 6);
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
