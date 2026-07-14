use super::*;

#[test]
#[cfg(target_os = "macos")]
pub(crate) fn compiled_same_process_executable_substitutions_are_denied_without_success_or_reuse() {
    let _serial = mediator_lock();
    for mode in [
        "user-owned-binary",
        "different-root-binary",
        "different-object-alias",
        "dev-fd-binary",
        "shebang-script",
    ] {
        let fixture = fixture(&format!("mediator-exec-substitution-{mode}"), true);
        let scope = fixture.repo.root().join("target/routine/syntax");
        let relative_scope = "target/routine/syntax";
        let mut alias_owner = None;
        let command = match mode {
            "user-owned-binary" => {
                let adversary = compile_containment_adversary(&fixture);
                format!(
                    "exec {} complete {}",
                    shell_literal(&adversary.to_string_lossy()),
                    shell_literal(relative_scope)
                )
            }
            "different-root-binary" => format!(
                "exec /bin/bash -c {}",
                shell_literal(&format!(
                    "printf unexpected > {relative_scope}/substitute.effect; {}",
                    successful_exit_script()
                ))
            ),
            "different-object-alias" => {
                let owner = TempRepo::new("mediator-different-object-alias-owner");
                let alias = owner.root().join("different-object-alias");
                std::os::unix::fs::symlink("/bin/bash", &alias).unwrap();
                let command = format!(
                    "exec {} -c {}",
                    shell_literal(&alias.to_string_lossy()),
                    shell_literal(&format!(
                        "printf unexpected > {relative_scope}/substitute.effect; {}",
                        successful_exit_script()
                    ))
                );
                alias_owner = Some(owner);
                command
            }
            "dev-fd-binary" => {
                let adversary = compile_containment_adversary(&fixture);
                format!(
                    "exec 9<{}; exec /dev/fd/9 complete {}",
                    shell_literal(&adversary.to_string_lossy()),
                    shell_literal(relative_scope)
                )
            }
            "shebang-script" => {
                let script = fixture
                    .repo
                    .root()
                    .join("target/routine/shebang-substitute");
                fs::write(
                    &script,
                    format!(
                        "#!/bin/dash\nprintf unexpected > \"$1/substitute.effect\"\n{}\n",
                        successful_exit_script()
                    ),
                )
                .unwrap();
                fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
                format!(
                    "exec {} {}",
                    shell_literal(&script.to_string_lossy()),
                    shell_literal(relative_scope)
                )
            }
            _ => unreachable!(),
        };
        let _alias_owner = &alias_owner;
        let prepared = prepared_with(
            &fixture,
            |node_id| executable_substitution_script(node_id, &command),
            10_000,
            1024 * 1024,
        );
        let grant = issue_grant(&prepared, &format!("exec-substitution-{mode}"), None);
        let result = mediate(
            &fixture,
            prepared,
            Some(grant),
            RoutineCancellation::new(),
            Vec::new(),
        );
        assert_eq!(
            result.status(),
            RoutineMediatorStatus::IncompleteExecution,
            "mode={mode} nodes={:?}",
            result.nodes()
        );
        assert_eq!(
            result.nodes()[0].failure_code(),
            Some("MEDIATOR-CHECK-FAILED"),
            "mode={mode} nodes={:?}",
            result.nodes()
        );
        assert!(
            result
                .nodes()
                .iter()
                .all(|node| node.result_artifact_sha256().is_none()),
            "mode={mode} exposed a result artifact"
        );
        assert!(result.reuse_artifacts().is_empty(), "mode={mode}");
        assert!(result.recovery_marker().is_some(), "mode={mode}");
        assert!(
            !scope.join("substitute.effect").exists(),
            "mode={mode} reached the substitute effect"
        );
        let process = read_reported_process(&scope).unwrap();
        assert_process_absent(process.pid);
        assert_process_group_absent(process.pgid);
        assert_output_stopped(&process.activity, mode);
    }
}

#[test]
#[cfg(target_os = "macos")]
pub(crate) fn exact_same_executable_reexec_remains_single_process_and_can_complete() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-exact-reexec", true);
    let prepared = prepared_with(
        &fixture,
        |node_id| {
            format!(
                "{}; exec /bin/dash -c {}",
                runner_record_script(node_id),
                shell_literal(&command_script(node_id))
            )
        },
        10_000,
        1024 * 1024,
    );
    let grant = issue_grant(&prepared, "exact-reexec-session", None);
    let result = mediate(
        &fixture,
        prepared,
        Some(grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(result.status(), RoutineMediatorStatus::CompleteExecution);
    assert_eq!(result.reuse_artifacts().len(), 3);
    for node_id in ["syntax", "compile", "unit"] {
        let scope = fixture
            .repo
            .root()
            .join(format!("target/routine/{node_id}"));
        let process = read_reported_process(&scope).unwrap();
        assert_process_absent(process.pid);
        assert_process_group_absent(process.pgid);
    }
}

#[test]
#[cfg(target_os = "macos")]
pub(crate) fn compiled_user_owned_dlopen_mapping_is_denied_without_effect_success_or_reuse() {
    let _serial = mediator_lock();
    let fixture = fixture_for_tool("mediator-user-dlopen", true, "ruby");
    let library = compile_mapping_adversary(&fixture);
    let root = fixture.repo.root().to_owned();
    let prepared = prepared_with_arguments(
        &fixture,
        |node_id| ruby_mapping_adversary_arguments(node_id, &library, &root),
        10_000,
        1024 * 1024,
    );
    let grant = issue_grant(&prepared, "user-dlopen-session", None);
    let result = mediate(
        &fixture,
        prepared,
        Some(grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(
        result.status(),
        RoutineMediatorStatus::IncompleteExecution,
        "nodes={:?}",
        result.nodes()
    );
    assert_eq!(
        result.nodes()[0].failure_code(),
        Some("MEDIATOR-CHECK-FAILED")
    );
    assert!(
        result
            .nodes()
            .iter()
            .all(|node| node.result_artifact_sha256().is_none())
    );
    assert!(result.reuse_artifacts().is_empty());
    assert!(result.recovery_marker().is_some());
    let scope = fixture.repo.root().join("target/routine/syntax");
    assert!(
        !scope.join("mapping.effect").exists(),
        "user-owned executable mapping reached its effect"
    );
    let process = read_reported_process(&scope).unwrap();
    assert_process_absent(process.pid);
    assert_process_group_absent(process.pgid);
    assert_output_stopped(&process.activity, "user-dlopen");
}

#[test]
#[cfg(target_os = "macos")]
pub(crate) fn pinned_single_process_route_completes_and_is_absent_after_natural_exit() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-compiled-single-process", true);
    let prepared = prepared_with(
        &fixture,
        |node_id| adversary_script(node_id, "complete"),
        10_000,
        1024 * 1024,
    );
    let grant = issue_grant(&prepared, "compiled-single-process", None);
    let result = mediate(
        &fixture,
        prepared,
        Some(grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(result.status(), RoutineMediatorStatus::CompleteExecution);
    assert_eq!(result.reuse_artifacts().len(), 3);
    for node_id in ["syntax", "compile", "unit"] {
        let scope = fixture
            .repo
            .root()
            .join(format!("target/routine/{node_id}"));
        let process = read_reported_process(&scope).unwrap();
        assert_process_absent(process.pid);
        assert_process_group_absent(process.pgid);
        assert_output_stopped(&process.activity, "compiled-natural-exit");
    }
}
