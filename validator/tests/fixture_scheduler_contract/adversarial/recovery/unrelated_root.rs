#[test]
fn recovery_refuses_an_unrelated_replacement_root_and_retains_the_lease() {
    let root = root("replacement-root");
    let mut scheduler = FixtureScheduler::new(&root);
    let id = scheduler
        .schedule([control("replacement-root", "replacement-root")])
        .unwrap()
        .pop()
        .unwrap();
    let lease_root = scheduler.run(&id).unwrap().lease.root().to_path_buf();
    let pinned_root = root.join("pinned-root");
    fs::rename(&lease_root, &pinned_root).unwrap();
    fs::create_dir(&lease_root).unwrap();
    let sentinel = lease_root.join("unrelated-sentinel");
    fs::write(&sentinel, b"must survive").unwrap();

    assert_eq!(
        scheduler.recover(&id).unwrap(),
        RunDisposition::CleanupFailure
    );
    assert_eq!(fs::read(&sentinel).unwrap(), b"must survive");
    assert_eq!(scheduler.active_count(), 1);

    fs::remove_dir_all(&lease_root).unwrap();
    fs::rename(&pinned_root, &lease_root).unwrap();
    assert_eq!(
        scheduler.recover(&id).unwrap(),
        repaired_recovery_disposition()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn false_pass_ordinary_finish_failure_is_recovery_required_before_reinsertion() {
    let root = root("ordinary-finish-recovery-state");
    let mut scheduler = FixtureScheduler::new(&root);
    let id = scheduler
        .schedule([control("ordinary-finish", "ordinary-finish")])
        .unwrap()
        .pop()
        .unwrap();
    let lease_root = scheduler.run(&id).unwrap().lease.root().to_path_buf();
    let unrelated = lease_root.join("0-unrelated-sentinel");
    fs::write(&unrelated, b"must survive").unwrap();

    assert_eq!(
        scheduler
            .finish(&id, ObservedOutcome::failure("ordinary-finish", 1))
            .unwrap(),
        RunDisposition::CleanupFailure
    );
    let retained = scheduler.run(&id).unwrap();
    assert_eq!(
        retained.lease.disposition(),
        &LeaseDisposition::RecoveryRequired
    );
    assert!(!retained.is_active());
    assert_eq!(fs::read(&unrelated).unwrap(), b"must survive");

    assert_eq!(
        scheduler.recover(&id).unwrap(),
        RunDisposition::CleanupFailure
    );
    assert_eq!(
        scheduler.run(&id).unwrap().lease.disposition(),
        &LeaseDisposition::RecoveryRequired
    );
    assert!(!scheduler.run(&id).unwrap().is_active());
    assert_eq!(fs::read(&unrelated).unwrap(), b"must survive");

    drop(scheduler);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn resource_and_mutation_finish_observed_failure_retains_recovery_state_and_bytes() {
    let root = root("finish-observed-recovery-state");
    let mut scheduler = FixtureScheduler::new(&root);
    let id = scheduler
        .schedule([control("finish-observed", "finish-observed")])
        .unwrap()
        .pop()
        .unwrap();
    let lease_root = scheduler.run(&id).unwrap().lease.root().to_path_buf();
    let owned_output = lease_root.join("process/owned-output");
    fs::write(&owned_output, b"fixture-owned-resource").unwrap();
    let unrelated = lease_root.join("0-unrelated-sentinel");
    fs::write(&unrelated, b"unrelated").unwrap();

    assert_eq!(
        scheduler
            .finish_observed(&id, ObservedOutcome::failure("finish-observed", 1))
            .unwrap(),
        RunDisposition::CleanupFailure
    );
    let retained = scheduler.run(&id).unwrap();
    assert_eq!(
        retained.lease.disposition(),
        &LeaseDisposition::RecoveryRequired
    );
    assert!(!retained.is_active());
    assert_eq!(fs::read(&owned_output).unwrap(), b"fixture-owned-resource");
    assert_eq!(fs::read(&unrelated).unwrap(), b"unrelated");

    assert_eq!(
        scheduler.recover(&id).unwrap(),
        RunDisposition::CleanupFailure
    );
    assert_eq!(
        scheduler.run(&id).unwrap().lease.disposition(),
        &LeaseDisposition::RecoveryRequired
    );
    assert_eq!(fs::read(&owned_output).unwrap(), b"fixture-owned-resource");

    drop(scheduler);
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn recovery_refuses_root_symlink_substitution_without_touching_outside_data() {
    use std::os::unix::fs::symlink;

    let outside = root("root-symlink-outside");
    let root = root("root-symlink-substitution");
    fs::create_dir_all(&outside).unwrap();
    let outside_sentinel = outside.join("outside-sentinel");
    fs::write(&outside_sentinel, b"outside").unwrap();
    let mut scheduler = FixtureScheduler::new(&root);
    let id = scheduler
        .schedule([control("root-symlink", "root-symlink")])
        .unwrap()
        .pop()
        .unwrap();
    let lease_root = scheduler.run(&id).unwrap().lease.root().to_path_buf();
    let pinned_root = root.join("pinned-root");
    fs::rename(&lease_root, &pinned_root).unwrap();
    symlink(&outside, &lease_root).unwrap();

    assert_eq!(
        scheduler.recover(&id).unwrap(),
        RunDisposition::CleanupFailure
    );
    assert_eq!(fs::read(&outside_sentinel).unwrap(), b"outside");

    fs::remove_file(&lease_root).unwrap();
    fs::rename(&pinned_root, &lease_root).unwrap();
    assert_eq!(
        scheduler.recover(&id).unwrap(),
        repaired_recovery_disposition()
    );
    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(outside).unwrap();
}

#[cfg(unix)]
#[test]
fn recovery_refuses_special_file_root_substitution() {
    use std::ffi::CString;

    let root = root("root-special-substitution");
    let mut scheduler = FixtureScheduler::new(&root);
    let id = scheduler
        .schedule([control("root-special", "root-special")])
        .unwrap()
        .pop()
        .unwrap();
    let lease_root = scheduler.run(&id).unwrap().lease.root().to_path_buf();
    let pinned_root = root.join("pinned-root");
    fs::rename(&lease_root, &pinned_root).unwrap();
    let raw = CString::new(lease_root.as_os_str().as_encoded_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(raw.as_ptr(), 0o600) }, 0);

    assert_eq!(
        scheduler.recover(&id).unwrap(),
        RunDisposition::CleanupFailure
    );

    fs::remove_file(&lease_root).unwrap();
    fs::rename(&pinned_root, &lease_root).unwrap();
    assert_eq!(
        scheduler.recover(&id).unwrap(),
        repaired_recovery_disposition()
    );
    fs::remove_dir_all(root).unwrap();
}
