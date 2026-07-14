fn root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "hul-fixture-{name}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}
fn expected(code: &str) -> ExpectedOutcome {
    ExpectedOutcome::causal_failure(code, 1)
}
fn control(id: &str, code: &str) -> FixtureSpec {
    FixtureSpec::new(
        id,
        FixtureKind::Negative,
        "lifecycle",
        BTreeSet::from([ResourceKind::Process]),
        expected(code),
        false,
    )
    .unwrap()
}

#[cfg(target_os = "freebsd")]
fn repaired_recovery_disposition() -> RunDisposition {
    RunDisposition::CausalFailure
}

#[cfg(not(target_os = "freebsd"))]
fn repaired_recovery_disposition() -> RunDisposition {
    RunDisposition::CleanupFailure
}

#[cfg(unix)]
fn cleanup_quarantine(parent: &std::path::Path) -> PathBuf {
    fs::read_dir(parent)
        .unwrap()
        .map(Result::unwrap)
        .find(|entry| {
            entry
                .file_name()
                .as_encoded_bytes()
                .starts_with(b".hul-delete-")
        })
        .unwrap()
        .path()
}

#[test]
fn altered_expectations_cannot_manufacture_a_pass() {
    let root = root("integrity");
    let mut forged = control("expectation-integrity", "wrong-output");
    forged.expected = ExpectedOutcome::pass(255);
    let mut scheduler = FixtureScheduler::new(&root);
    assert!(matches!(
        scheduler.schedule([forged]),
        Err(FixtureScheduleError::Integrity(_))
    ));
}

#[test]
fn timeout_signal_output_and_text_attacks_keep_their_causal_reason() {
    let root = root("lifecycle");
    let mut scheduler = FixtureScheduler::new(&root);
    for code in [
        "timeout",
        "signal",
        "output-flood",
        "non-utf8",
        "spoofed-text",
        "wrong-output",
        "race",
        "interruption",
    ] {
        let id = scheduler
            .schedule([control(code, code)])
            .unwrap()
            .pop()
            .unwrap();
        let result = scheduler.finish(&id, ObservedOutcome::failure(code, 1));
        #[cfg(target_os = "freebsd")]
        assert!(matches!(result, Err(FixtureScheduleError::Integrity(_))));
        #[cfg(not(target_os = "freebsd"))]
        assert_eq!(result.unwrap(), RunDisposition::CleanupFailure);
    }
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn special_file_cleanup_failure_is_recoverable() {
    use std::ffi::CString;
    let root = root("special-file");
    let mut scheduler = FixtureScheduler::new(&root);
    let id = scheduler
        .schedule([control("special-file", "special-file")])
        .unwrap()
        .pop()
        .unwrap();
    let lease_root = scheduler.run(&id).unwrap().lease.root().to_path_buf();
    fs::remove_file(lease_root.join(".fixture-lease")).unwrap();
    let fifo = lease_root.join("fifo");
    let fifo_raw = CString::new(fifo.as_os_str().as_encoded_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo_raw.as_ptr(), 0o600) }, 0);
    assert_eq!(
        scheduler
            .finish(&id, ObservedOutcome::failure("special-file", 1))
            .unwrap(),
        RunDisposition::CleanupFailure
    );
    assert_eq!(
        scheduler.run(&id).unwrap().lease.disposition(),
        &LeaseDisposition::RecoveryRequired
    );
    assert!(!scheduler.run(&id).unwrap().is_active());
    fs::remove_file(&fifo).unwrap();
    assert_eq!(
        scheduler.recover(&id).unwrap(),
        repaired_recovery_disposition()
    );
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn symlink_cleanup_failure_is_causal_and_does_not_hide_the_lease() {
    use std::os::unix::fs::symlink;
    let root = root("symlink");
    let mut scheduler = FixtureScheduler::new(&root);
    let id = scheduler
        .schedule([control("symlink", "symlink")])
        .unwrap()
        .pop()
        .unwrap();
    let lease_root = scheduler.run(&id).unwrap().lease.root().to_path_buf();
    fs::remove_file(lease_root.join(".fixture-lease")).unwrap();
    symlink("/tmp", lease_root.join("escape")).unwrap();
    assert_eq!(
        scheduler
            .finish(&id, ObservedOutcome::failure("symlink", 1))
            .unwrap(),
        RunDisposition::CleanupFailure
    );
    assert_eq!(
        scheduler.run(&id).unwrap().lease.disposition(),
        &LeaseDisposition::RecoveryRequired
    );
    assert!(!scheduler.run(&id).unwrap().is_active());
    assert!(lease_root.exists());
    fs::remove_file(lease_root.join("escape")).unwrap();
    assert_eq!(
        scheduler.recover(&id).unwrap(),
        repaired_recovery_disposition()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn wrong_outcome_is_rejected_after_cleanup() {
    let root = root("wrong-outcome");
    let mut scheduler = FixtureScheduler::new(&root);
    let id = scheduler
        .schedule([control("race", "race")])
        .unwrap()
        .pop()
        .unwrap();
    let result = scheduler.finish(&id, ObservedOutcome::failure("wrong-output", 1));
    #[cfg(target_os = "freebsd")]
    {
        assert!(matches!(result, Err(FixtureScheduleError::Integrity(_))));
        assert_eq!(scheduler.active_count(), 0);
    }
    #[cfg(not(target_os = "freebsd"))]
    {
        assert_eq!(result.unwrap(), RunDisposition::CleanupFailure);
        assert_eq!(scheduler.active_count(), 1);
    }
    fs::remove_dir_all(root).unwrap();
}
