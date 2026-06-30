use std::path::PathBuf;

fn raw(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

#[test]
fn routine_help_fit_repo_and_target_repo_paths_are_discoverable() {
    let help = crate::cli::usage::text();
    for expected in [
        "Routine validation",
        "ultragoal routine check",
        "First plugin-activated repo path",
        "ultragoal fit-repo prove",
        "Target repo path",
        "ultragoal target-repo audit",
        "scripts/check is a narrow helper",
        "unsupported claims",
    ] {
        assert!(help.contains(expected), "{expected}");
    }
    assert!(matches!(
        crate::parse_command(&raw(&["--help"])).expect("help"),
        crate::Command::Help
    ));
    assert!(matches!(
        crate::parse_command(&raw(&["routine", "check"])).expect("routine"),
        crate::Command::Routine(_)
    ));
    assert!(matches!(
        crate::parse_command(&raw(&[
            "fit-repo",
            "prove",
            "--receipt-dir",
            "validation_artifacts/harness"
        ]))
        .expect("fit-repo"),
        crate::Command::Product(_)
    ));
    assert!(matches!(
        crate::parse_command(&raw(&["target-repo", "audit"])).expect("target"),
        crate::Command::Control(_)
    ));
}

#[test]
fn routine_command_writes_source_local_claim_ceiling_receipt() {
    let root = crate::self_tests::boundaries::support::temp_root("routine-command");
    std::fs::create_dir_all(root.join("scripts")).expect("scripts");
    std::fs::write(
        root.join("scripts/check"),
        "narrow-helper ceiling; unsupported claims; update_goal blocked",
    )
    .expect("script");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &serde_json::json!({"resources":["scripts/check"]}),
    )
    .expect("manifest");
    let receipt = PathBuf::from("validation_artifacts/cli/routine.json");
    let code = crate::command_run::run_with_exit_code(crate::Args {
        root: root.clone(),
        command: crate::Command::Routine(crate::cli::routine::RoutineCommand {
            receipt: receipt.clone(),
            target_repo: Some(PathBuf::from("target-repo")),
        }),
    })
    .expect("run routine");
    assert_eq!(code, 0);
    let value = crate::json_boundary::read_json(&root.join(receipt)).expect("receipt");
    assert_eq!(value["status"], "pass");
    assert_eq!(
        value["claim_ceiling"],
        "routine_usability_only_not_readiness"
    );
    assert!(
        value["blocked_claim_classes"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("update_goal_eligibility"))
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}
