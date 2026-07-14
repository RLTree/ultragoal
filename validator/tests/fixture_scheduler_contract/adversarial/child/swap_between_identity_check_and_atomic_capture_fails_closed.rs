#[cfg(unix)]
#[test]
fn child_swap_between_identity_check_and_atomic_capture_fails_closed() {
    use std::cell::Cell;
    use std::rc::Rc;

    let root = root("child-capture-race");
    let mut scheduler = FixtureScheduler::new(&root);
    let id = scheduler
        .schedule([control("capture-race-child", "capture-race-child")])
        .unwrap()
        .pop()
        .unwrap();
    let lease_root = scheduler.run(&id).unwrap().lease.root().to_path_buf();
    fs::remove_file(lease_root.join(".fixture-lease")).unwrap();
    let child = lease_root.join("process");
    let pinned_child = root.join("pinned-child-after-check");
    let fired = Rc::new(Cell::new(false));
    let fired_for_hook = Rc::clone(&fired);
    let child_for_hook = child.clone();
    let pinned_child_for_hook = pinned_child.clone();
    set_before_capture_hook(Some(Box::new(move |name| {
        if !fired_for_hook.get() && name.as_encoded_bytes() == b"process" {
            fs::rename(&child_for_hook, &pinned_child_for_hook).unwrap();
            fs::create_dir(&child_for_hook).unwrap();
            fs::write(child_for_hook.join("unrelated-sentinel"), b"must survive").unwrap();
            fired_for_hook.set(true);
        }
    })));

    assert_eq!(
        scheduler.recover(&id).unwrap(),
        RunDisposition::CleanupFailure
    );
    set_before_capture_hook(None);
    assert!(fired.get());
    #[cfg(target_os = "freebsd")]
    let replacement = child.clone();
    #[cfg(not(target_os = "freebsd"))]
    let replacement = cleanup_quarantine(&lease_root);
    assert_eq!(
        fs::read(replacement.join("unrelated-sentinel")).unwrap(),
        b"must survive"
    );

    fs::remove_dir_all(&replacement).unwrap();
    fs::rename(&pinned_child, &child).unwrap();
    assert_eq!(
        scheduler.recover(&id).unwrap(),
        repaired_recovery_disposition()
    );
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn dynamic_file_swap_between_identity_check_and_atomic_capture_fails_closed() {
    use std::cell::Cell;
    use std::rc::Rc;

    let root = root("dynamic-file-capture-race");
    let mut scheduler = FixtureScheduler::new(&root);
    let id = scheduler
        .schedule([control("capture-race-file", "capture-race-file")])
        .unwrap()
        .pop()
        .unwrap();
    let lease_root = scheduler.run(&id).unwrap().lease.root().to_path_buf();
    fs::remove_file(lease_root.join(".fixture-lease")).unwrap();
    let file = lease_root.join("process/owned-output");
    fs::write(&file, b"fixture output").unwrap();
    let pinned_file = root.join("pinned-file-after-check");
    let fired = Rc::new(Cell::new(false));
    let fired_for_hook = Rc::clone(&fired);
    let file_for_hook = file.clone();
    let pinned_file_for_hook = pinned_file.clone();
    set_before_capture_hook(Some(Box::new(move |name| {
        if !fired_for_hook.get() && name.as_encoded_bytes() == b"owned-output" {
            fs::rename(&file_for_hook, &pinned_file_for_hook).unwrap();
            fs::write(&file_for_hook, b"unrelated replacement").unwrap();
            fired_for_hook.set(true);
        }
    })));

    assert_eq!(
        scheduler.recover(&id).unwrap(),
        RunDisposition::CleanupFailure
    );
    set_before_capture_hook(None);
    assert!(fired.get());
    #[cfg(target_os = "freebsd")]
    let replacement = file.clone();
    #[cfg(not(target_os = "freebsd"))]
    let replacement = cleanup_quarantine(file.parent().unwrap());
    assert_eq!(fs::read(&replacement).unwrap(), b"unrelated replacement");

    fs::remove_file(&replacement).unwrap();
    fs::rename(&pinned_file, &file).unwrap();
    assert_eq!(
        scheduler.recover(&id).unwrap(),
        repaired_recovery_disposition()
    );
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
fn filesystem_identity(path: &std::path::Path) -> (u64, u64) {
    use std::os::unix::fs::MetadataExt;

    let metadata = fs::symlink_metadata(path).unwrap();
    (metadata.dev(), metadata.ino())
}

#[cfg(unix)]
#[test]
fn child_file_swap_after_capture_validation_retains_both_files() {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    let root = root("child-file-final-unlink-race");
    let mut scheduler = FixtureScheduler::new(&root);
    let id = scheduler
        .schedule([control("final-unlink-file", "final-unlink-file")])
        .unwrap()
        .pop()
        .unwrap();
    let lease_root = scheduler.run(&id).unwrap().lease.root().to_path_buf();
    let marker = lease_root.join(".fixture-lease");
    let expected_identity = filesystem_identity(&marker);
    let retained_expected = root.join("retained-expected-marker");
    let replacement_path = Rc::new(RefCell::new(None::<PathBuf>));
    let replacement_path_for_hook = Rc::clone(&replacement_path);
    let armed = Rc::new(Cell::new(false));
    let armed_for_hook = Rc::clone(&armed);
    let fired = Rc::new(Cell::new(false));
    let fired_for_hook = Rc::clone(&fired);
    let lease_root_for_hook = lease_root.clone();
    let retained_expected_for_hook = retained_expected.clone();
    set_before_capture_hook(Some(Box::new(move |name| {
        if !armed_for_hook.get() && name.as_encoded_bytes() == b".fixture-lease" {
            armed_for_hook.set(true);
        } else if armed_for_hook.get()
            && !fired_for_hook.get()
            && (name.as_encoded_bytes() == b".fixture-lease"
                || name.as_encoded_bytes().starts_with(b".hul-delete-"))
        {
            let replacement = lease_root_for_hook.join(name);
            fs::rename(&replacement, &retained_expected_for_hook).unwrap();
            fs::write(&replacement, b"unrelated replacement").unwrap();
            *replacement_path_for_hook.borrow_mut() = Some(replacement);
            fired_for_hook.set(true);
        }
    })));

    let disposition = scheduler
        .finish(&id, ObservedOutcome::failure("final-unlink-file", 1))
        .unwrap();
    set_before_capture_hook(None);
    let retained_disposition = scheduler.run(&id).unwrap().lease.disposition().clone();
    let retained_is_active = scheduler.run(&id).unwrap().is_active();
    let replacement = replacement_path.borrow().clone();
    let retained_identity = retained_expected
        .exists()
        .then(|| filesystem_identity(&retained_expected));
    let replacement_bytes = replacement.as_ref().and_then(|path| fs::read(path).ok());
    drop(scheduler);
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }

    assert_eq!(disposition, RunDisposition::CleanupFailure);
    assert_eq!(retained_disposition, LeaseDisposition::RecoveryRequired);
    assert!(!retained_is_active);
    assert!(fired.get());
    assert_eq!(retained_identity, Some(expected_identity));
    assert_eq!(
        replacement_bytes.as_deref(),
        Some(b"unrelated replacement".as_slice())
    );
}
