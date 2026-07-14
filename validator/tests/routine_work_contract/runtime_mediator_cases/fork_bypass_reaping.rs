use super::*;

#[test]
#[cfg(target_os = "macos")]
pub(crate) fn fork_family_bypasses_are_killed_before_success_or_reuse() {
    let _serial = mediator_lock();
    for mode in [
        "fork-new-pgid",
        "fork-new-session",
        "posix-spawn",
        "vfork",
        "raw-fork",
    ] {
        let fixture = fixture_for_tool(&format!("mediator-{mode}"), true, "ruby");
        let prepared = prepared_with_arguments(
            &fixture,
            |node_id| ruby_process_creation_arguments(node_id, mode),
            10_000,
            1024 * 1024,
        );
        let grant = issue_grant(&prepared, &format!("{mode}-session"), None);
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
            Some("MEDIATOR-CHECK-FAILED")
        );
        assert!(result.reuse_artifacts().is_empty());
        assert!(result.recovery_marker().is_some());

        let scope = fixture.repo.root().join("target/routine/syntax");
        let process = read_reported_process(&scope).unwrap();
        assert!(
            scope.join("report.emitted").exists(),
            "mode={mode} did not reach the valid-report control before fork"
        );
        assert!(
            !scope.join("attempt.returned").exists(),
            "mode={mode} process-creation denial returned instead of killing the adversary"
        );
        assert!(
            !scope.join("process-creation.succeeded").exists(),
            "mode={mode} process creation reached a success effect"
        );
        assert!(
            !scope.join("created-child.pid").exists(),
            "mode={mode} process creation returned a child PID"
        );
        for child_record in ["child.pid", "child.pgid", "child.sid"] {
            assert!(
                !scope.join(child_record).exists(),
                "mode={mode} created descendant record {child_record}"
            );
        }
        assert_process_absent(process.pid);
        assert_process_group_absent(process.pgid);
        assert_output_stopped(&process.activity, mode);
    }
}

#[test]
#[cfg(target_os = "macos")]
pub(crate) fn group_leader_joining_existing_pgid_is_reaped_by_direct_pid() {
    let _serial = mediator_lock();
    let target_pgid = unsafe { libc::getpgrp() };
    assert!(target_pgid > 0);
    let fixture = fixture_for_tool("mediator-join-existing-pgid", true, "ruby");
    let prepared = prepared_with_arguments(
        &fixture,
        |node_id| ruby_join_existing_pgid_arguments(node_id, target_pgid),
        10_000,
        1024 * 1024,
    );
    let grant = issue_grant(&prepared, "join-existing-pgid-session", None);
    let cancellation = RoutineCancellation::new();
    let trigger = cancellation.clone();
    let scope = fixture.repo.root().join("target/routine/syntax");
    let verifier_scope = scope.clone();
    let verifier = std::thread::spawn(move || {
        let process = wait_for_joined_process(&verifier_scope, target_pgid);
        trigger.cancel();
        process
    });
    let result = mediate(&fixture, prepared, Some(grant), cancellation, Vec::new());
    let process = verifier.join().unwrap();
    assert_eq!(result.status(), RoutineMediatorStatus::Cancelled);
    assert_eq!(result.nodes()[0].failure_code(), Some("MEDIATOR-CANCELLED"));
    assert!(
        result
            .nodes()
            .iter()
            .all(|node| node.result_artifact_sha256().is_none())
    );
    assert!(result.reuse_artifacts().is_empty());
    assert!(result.recovery_marker().is_some());
    assert_eq!(
        fs::read_to_string(scope.join("join.actual-pgid"))
            .unwrap()
            .trim()
            .parse::<i32>()
            .unwrap(),
        target_pgid
    );
    assert_ne!(process.pgid, target_pgid);
    assert_process_absent(process.pid);
    assert_process_group_absent(process.pgid);
    assert_output_stopped(&process.activity, "join-existing-pgid");
}

#[test]
#[cfg(target_os = "macos")]
pub(crate) fn term_resistant_output_overflow_is_reaped_without_reuse() {
    let _serial = mediator_lock();
    let overflow_fixture = fixture("mediator-output-overflow", true);
    let prepared = prepared_with(
        &overflow_fixture,
        |node_id| adversary_script(node_id, "overflow"),
        10_000,
        4096,
    );
    let grant = issue_grant(&prepared, "overflow-session", None);
    let overflow = mediate(
        &overflow_fixture,
        prepared,
        Some(grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(
        overflow.status(),
        RoutineMediatorStatus::IncompleteExecution
    );
    assert_eq!(
        overflow.nodes()[0].failure_code(),
        Some("MEDIATOR-OUTPUT-LIMIT")
    );
    assert!(overflow.reuse_artifacts().is_empty());
    let scope = overflow_fixture.repo.root().join("target/routine/syntax");
    let process = read_reported_process(&scope).unwrap();
    assert_process_absent(process.pid);
    assert_process_group_absent(process.pgid);
    assert_output_stopped(&process.activity, "output-limit");
}

#[cfg(unix)]
pub(crate) fn assert_process_absent(pid: i32) {
    let status = unsafe { libc::kill(pid, 0) };
    let error = std::io::Error::last_os_error();
    assert_eq!(status, -1, "process {pid} survived cleanup");
    assert_eq!(error.raw_os_error(), Some(libc::ESRCH));
}

#[cfg(unix)]
pub(crate) fn assert_process_group_absent(pgid: i32) {
    assert!(pgid > 0, "reported PGID must be strictly positive: {pgid}");
    let target = pgid
        .checked_neg()
        .expect("strictly positive PGID has a negative signal target");
    let status = unsafe { libc::kill(target, 0) };
    let error = std::io::Error::last_os_error();
    assert_eq!(status, -1, "process group {pgid} survived cleanup");
    assert_eq!(error.raw_os_error(), Some(libc::ESRCH));
}

pub(crate) fn assert_output_stopped(activity: &Path, label: &str) {
    let stopped = fs::read(activity).unwrap();
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(
        fs::read(activity).unwrap(),
        stopped,
        "case={label} output continued after cleanup"
    );
}

#[test]
pub(crate) fn mediator_source_keeps_issuance_public_dispatch_and_claims_outside_the_boundary() {
    let mediator = include_str!("../../src/routine_work/runtime_adapter/mediator/mod.rs");
    let model = include_str!("../../src/routine_work/runtime_adapter/mediator/model.rs");
    let process = include_str!("../../src/routine_work/runtime_adapter/mediator/process/mod.rs");
    let filesystem =
        include_str!("../../src/routine_work/runtime_adapter/mediator/filesystem/mod.rs");
    let production_filesystem = filesystem.split("#[cfg(all(test, unix))]").next().unwrap();
    assert!(mediator.contains("grant.ok_or_else"));
    assert!(mediator.contains("reserve_grant"));
    assert!(mediator.contains("validate_snapshot"));
    assert!(model.contains("Production construction is intentionally absent"));
    assert!(model.contains("#[cfg(test)]\nimpl RoutineRootGrant"));
    assert!(!mediator.contains("ClaimDecision"));
    assert!(!mediator.contains("public command"));
    assert!(process.contains("(deny network*)"));
    assert!(process.contains("(deny process-fork (with send-signal SIGKILL))"));
    assert!(!process.contains("(allow process-fork"));
    assert!(process.contains("(deny process-exec)"));
    assert!(process.contains("(allow process-exec (literal"));
    assert!(process.contains("let profile = sandbox_profile("));
    assert!(process.contains("&reads.absolute_sources()"));
    assert!(process.contains("(deny file-map-executable)"));
    assert!(process.contains(r#"file-map-executable (subpath \"/System\")"#));
    assert!(process.contains(r#"file-map-executable (subpath \"/usr/lib\")"#));
    assert!(process.contains("(deny file-read*)"));
    assert!(process.contains("(allow file-read* (literal"));
    assert!(process.contains(r#"file-read* (subpath \"/System\")"#));
    assert!(process.contains(r#"file-read* (subpath \"/usr/lib\")"#));
    assert!(process.contains("(deny file-write*)"));
    assert!(process.contains("setpgid(0, 0)"));
    assert!(process.contains("libc::kill(group.signal_target()?, signal)"));
    assert!(process.contains("libc::kill(group.signal_target()?, 0)"));
    assert!(!process.contains("libc::kill(group, signal)"));
    assert!(!process.contains("libc::kill(group, 0)"));
    assert!(process.contains("SpawnSetupGuard"));
    assert!(process.contains("RunningProcess"));
    assert!(mediator.contains("only the exact pinned executable identity may execute"));
    assert!(mediator.contains("unbound file reads are denied"));
    assert!(mediator.contains("identity/content/ctime revalidated"));
    assert!(mediator.contains("read_authority_sha256"));
    assert!(mediator.contains("user-owned executable mappings"));
    assert!(mediator.contains("multi-process runners are unsupported"));
    assert!(filesystem.contains("O_NOFOLLOW"));
    assert!(filesystem.contains("metadata.nlink() != 1"));
    assert!(production_filesystem.contains("libc::geteuid()"));
    assert!(production_filesystem.contains("metadata.uid() == effective_user_id"));
    assert!(production_filesystem.contains("reject_effective_user_control"));
    assert!(production_filesystem.contains("libc::faccessat"));
    assert!(production_filesystem.contains("libc::AT_EACCESS"));
    assert!(!production_filesystem.contains("libc::access(encoded.as_ptr(), libc::W_OK)"));
}
