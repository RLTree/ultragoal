#[cfg(unix)]
#[test]
fn child_directory_swap_after_capture_validation_retains_both_directories() {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    let root = root("child-directory-final-unlink-race");
    let mut scheduler = FixtureScheduler::new(&root);
    let id = scheduler
        .schedule([control("final-unlink-directory", "final-unlink-directory")])
        .unwrap()
        .pop()
        .unwrap();
    let lease_root = scheduler.run(&id).unwrap().lease.root().to_path_buf();
    fs::remove_file(lease_root.join(".fixture-lease")).unwrap();
    let child = lease_root.join("process");
    let expected_identity = filesystem_identity(&child);
    let retained_expected = root.join("retained-expected-directory");
    let replacement_path = Rc::new(RefCell::new(None::<PathBuf>));
    let replacement_path_for_hook = Rc::clone(&replacement_path);
    let armed = Rc::new(Cell::new(false));
    let armed_for_hook = Rc::clone(&armed);
    let fired = Rc::new(Cell::new(false));
    let fired_for_hook = Rc::clone(&fired);
    let lease_root_for_hook = lease_root.clone();
    let retained_expected_for_hook = retained_expected.clone();
    set_before_capture_hook(Some(Box::new(move |name| {
        if !armed_for_hook.get() && name.as_encoded_bytes() == b"process" {
            armed_for_hook.set(true);
        } else if armed_for_hook.get()
            && !fired_for_hook.get()
            && (name.as_encoded_bytes() == b"process"
                || name.as_encoded_bytes().starts_with(b".hul-delete-"))
        {
            let replacement = lease_root_for_hook.join(name);
            fs::rename(&replacement, &retained_expected_for_hook).unwrap();
            fs::create_dir(&replacement).unwrap();
            fs::write(replacement.join("unrelated-sentinel"), b"unrelated").unwrap();
            *replacement_path_for_hook.borrow_mut() = Some(replacement);
            fired_for_hook.set(true);
        }
    })));

    let disposition = scheduler
        .finish_observed(&id, ObservedOutcome::failure("final-unlink-directory", 1))
        .unwrap();
    set_before_capture_hook(None);
    let retained_disposition = scheduler.run(&id).unwrap().lease.disposition().clone();
    let retained_is_active = scheduler.run(&id).unwrap().is_active();
    let replacement = replacement_path.borrow().clone();
    let retained_identity = retained_expected
        .exists()
        .then(|| filesystem_identity(&retained_expected));
    let replacement_bytes = replacement
        .as_ref()
        .and_then(|path| fs::read(path.join("unrelated-sentinel")).ok());
    drop(scheduler);
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }

    assert_eq!(disposition, RunDisposition::CleanupFailure);
    assert_eq!(retained_disposition, LeaseDisposition::RecoveryRequired);
    assert!(!retained_is_active);
    assert!(fired.get());
    assert_eq!(retained_identity, Some(expected_identity));
    assert_eq!(replacement_bytes.as_deref(), Some(b"unrelated".as_slice()));
}

#[cfg(unix)]
#[test]
fn lease_root_swap_after_capture_validation_retains_both_directories() {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    let root = root("lease-root-final-unlink-race");
    let mut scheduler = FixtureScheduler::new(&root);
    let id = scheduler
        .schedule([control("final-unlink-root", "final-unlink-root")])
        .unwrap()
        .pop()
        .unwrap();
    let lease_root = scheduler.run(&id).unwrap().lease.root().to_path_buf();
    fs::remove_file(lease_root.join(".fixture-lease")).unwrap();
    fs::remove_dir(lease_root.join("process")).unwrap();
    let expected_identity = filesystem_identity(&lease_root);
    let lease_name = lease_root.file_name().unwrap().to_owned();
    let retained_expected = root.join("retained-expected-root");
    let replacement_path = Rc::new(RefCell::new(None::<PathBuf>));
    let replacement_path_for_hook = Rc::clone(&replacement_path);
    let armed = Rc::new(Cell::new(false));
    let armed_for_hook = Rc::clone(&armed);
    let fired = Rc::new(Cell::new(false));
    let fired_for_hook = Rc::clone(&fired);
    let root_for_hook = root.clone();
    let retained_expected_for_hook = retained_expected.clone();
    set_before_capture_hook(Some(Box::new(move |name| {
        if !armed_for_hook.get() && name == lease_name {
            armed_for_hook.set(true);
        } else if armed_for_hook.get()
            && !fired_for_hook.get()
            && (name == lease_name || name.as_encoded_bytes().starts_with(b".hul-delete-"))
        {
            let replacement = root_for_hook.join(name);
            fs::rename(&replacement, &retained_expected_for_hook).unwrap();
            fs::create_dir(&replacement).unwrap();
            fs::write(replacement.join("unrelated-sentinel"), b"unrelated").unwrap();
            *replacement_path_for_hook.borrow_mut() = Some(replacement);
            fired_for_hook.set(true);
        }
    })));

    let disposition = scheduler.recover(&id).unwrap();
    set_before_capture_hook(None);
    let retained_disposition = scheduler.run(&id).unwrap().lease.disposition().clone();
    let retained_is_active = scheduler.run(&id).unwrap().is_active();
    let replacement = replacement_path.borrow().clone();
    let retained_identity = retained_expected
        .exists()
        .then(|| filesystem_identity(&retained_expected));
    let replacement_bytes = replacement
        .as_ref()
        .and_then(|path| fs::read(path.join("unrelated-sentinel")).ok());
    drop(scheduler);
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }

    assert_eq!(disposition, RunDisposition::CleanupFailure);
    assert_eq!(retained_disposition, LeaseDisposition::RecoveryRequired);
    assert!(!retained_is_active);
    assert!(fired.get());
    assert_eq!(retained_identity, Some(expected_identity));
    assert_eq!(replacement_bytes.as_deref(), Some(b"unrelated".as_slice()));
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn unsupported_ordinary_cleanup_and_repeated_recovery_stay_typed_and_recoverable() {
    let root = root("unsupported-identity-delete");
    let spec = control("unsupported-delete", "unsupported-delete");
    let mut lease = IsolationLease::acquire(&root, &spec, 1).unwrap();
    let lease_root = lease.root().to_path_buf();
    let marker = lease_root.join(".fixture-lease");
    let expected_identity = filesystem_identity(&marker);

    let result = lease.cleanup();
    let retained_identity = marker.exists().then(|| filesystem_identity(&marker));
    let disposition = lease.disposition().clone();
    let repeated = lease.recover();
    let repeated_disposition = lease.disposition().clone();
    drop(lease);
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }

    let error = result.unwrap_err();
    match error {
        FixtureScheduleError::Cleanup { source, .. } => {
            assert_eq!(source.kind(), std::io::ErrorKind::Unsupported);
        }
        other => panic!("expected typed cleanup error, observed {other:?}"),
    }
    assert_eq!(disposition, LeaseDisposition::RecoveryRequired);
    assert!(matches!(
        repeated,
        Err(FixtureScheduleError::Cleanup { .. })
    ));
    assert_eq!(repeated_disposition, LeaseDisposition::RecoveryRequired);
    assert_eq!(retained_identity, Some(expected_identity));
}

#[cfg(target_os = "freebsd")]
#[test]
fn freebsd_identity_conditioned_delete_cleans_an_untampered_lease() {
    let root = root("freebsd-identity-delete");
    let spec = control("supported-delete", "supported-delete");
    let mut lease = IsolationLease::acquire(&root, &spec, 1).unwrap();
    let lease_root = lease.root().to_path_buf();

    lease.cleanup().unwrap();
    lease.recover().unwrap();

    assert_eq!(lease.disposition(), &LeaseDisposition::Cleaned);
    assert!(!lease_root.exists());
    fs::remove_dir_all(root).unwrap();
}
