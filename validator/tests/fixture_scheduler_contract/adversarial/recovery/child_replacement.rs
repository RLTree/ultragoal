#[test]
fn recovery_refuses_initial_child_directory_replacement() {
    let root = root("child-directory-replacement");
    let mut scheduler = FixtureScheduler::new(&root);
    let id = scheduler
        .schedule([control("child-directory", "child-directory")])
        .unwrap()
        .pop()
        .unwrap();
    let lease_root = scheduler.run(&id).unwrap().lease.root().to_path_buf();
    fs::remove_file(lease_root.join(".fixture-lease")).unwrap();
    let child = lease_root.join("process");
    let pinned_child = root.join("pinned-process-directory");
    fs::rename(&child, &pinned_child).unwrap();
    fs::create_dir(&child).unwrap();
    let sentinel = child.join("unrelated-sentinel");
    fs::write(&sentinel, b"must survive").unwrap();

    assert_eq!(
        scheduler.recover(&id).unwrap(),
        RunDisposition::CleanupFailure
    );
    assert_eq!(fs::read(&sentinel).unwrap(), b"must survive");

    fs::remove_dir_all(&child).unwrap();
    fs::rename(&pinned_child, &child).unwrap();
    assert_eq!(
        scheduler.recover(&id).unwrap(),
        repaired_recovery_disposition()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn recovery_refuses_initial_child_file_replacement() {
    let root = root("child-file-replacement");
    let mut scheduler = FixtureScheduler::new(&root);
    let id = scheduler
        .schedule([control("child-file", "child-file")])
        .unwrap()
        .pop()
        .unwrap();
    let lease_root = scheduler.run(&id).unwrap().lease.root().to_path_buf();
    let marker = lease_root.join(".fixture-lease");
    let pinned_marker = root.join("pinned-marker");
    fs::rename(&marker, &pinned_marker).unwrap();
    fs::write(&marker, b"unrelated sentinel").unwrap();

    assert_eq!(
        scheduler.recover(&id).unwrap(),
        RunDisposition::CleanupFailure
    );
    assert_eq!(fs::read(&marker).unwrap(), b"unrelated sentinel");

    fs::remove_file(&marker).unwrap();
    fs::rename(&pinned_marker, &marker).unwrap();
    assert_eq!(
        scheduler.recover(&id).unwrap(),
        repaired_recovery_disposition()
    );
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn recovery_refuses_initial_child_symlink_replacement() {
    use std::os::unix::fs::symlink;

    let outside = root("child-symlink-outside");
    let root = root("child-symlink-replacement");
    fs::create_dir_all(&outside).unwrap();
    let sentinel = outside.join("outside-sentinel");
    fs::write(&sentinel, b"outside").unwrap();
    let mut scheduler = FixtureScheduler::new(&root);
    let id = scheduler
        .schedule([control("child-symlink", "child-symlink")])
        .unwrap()
        .pop()
        .unwrap();
    let lease_root = scheduler.run(&id).unwrap().lease.root().to_path_buf();
    fs::remove_file(lease_root.join(".fixture-lease")).unwrap();
    let child = lease_root.join("process");
    let pinned_child = root.join("pinned-process-directory");
    fs::rename(&child, &pinned_child).unwrap();
    symlink(&outside, &child).unwrap();

    assert_eq!(
        scheduler.recover(&id).unwrap(),
        RunDisposition::CleanupFailure
    );
    assert_eq!(fs::read(&sentinel).unwrap(), b"outside");

    fs::remove_file(&child).unwrap();
    fs::rename(&pinned_child, &child).unwrap();
    assert_eq!(
        scheduler.recover(&id).unwrap(),
        repaired_recovery_disposition()
    );
    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(outside).unwrap();
}

#[test]
fn recovery_refuses_unexpected_top_level_regular_files() {
    let root = root("unexpected-top-level-file");
    let mut scheduler = FixtureScheduler::new(&root);
    let id = scheduler
        .schedule([control("unexpected-file", "unexpected-file")])
        .unwrap()
        .pop()
        .unwrap();
    let lease_root = scheduler.run(&id).unwrap().lease.root().to_path_buf();
    fs::remove_file(lease_root.join(".fixture-lease")).unwrap();
    fs::remove_dir(lease_root.join("process")).unwrap();
    let sentinel = lease_root.join("unrelated-sentinel");
    fs::write(&sentinel, b"must survive").unwrap();

    assert_eq!(
        scheduler.recover(&id).unwrap(),
        RunDisposition::CleanupFailure
    );
    assert_eq!(fs::read(&sentinel).unwrap(), b"must survive");

    fs::remove_file(&sentinel).unwrap();
    assert_eq!(
        scheduler.recover(&id).unwrap(),
        repaired_recovery_disposition()
    );
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn root_swap_between_identity_check_and_atomic_capture_fails_closed() {
    use std::cell::Cell;
    use std::rc::Rc;

    let root = root("root-capture-race");
    let mut scheduler = FixtureScheduler::new(&root);
    let id = scheduler
        .schedule([control("capture-race-root", "capture-race-root")])
        .unwrap()
        .pop()
        .unwrap();
    let lease_root = scheduler.run(&id).unwrap().lease.root().to_path_buf();
    fs::remove_file(lease_root.join(".fixture-lease")).unwrap();
    fs::remove_dir(lease_root.join("process")).unwrap();
    let pinned_root = root.join("pinned-root-after-check");
    let fired = Rc::new(Cell::new(false));
    let fired_for_hook = Rc::clone(&fired);
    let lease_root_for_hook = lease_root.clone();
    let pinned_root_for_hook = pinned_root.clone();
    set_before_capture_hook(Some(Box::new(move |name| {
        if !fired_for_hook.get() && name.as_encoded_bytes().starts_with(b"capture-race-root-") {
            fs::rename(&lease_root_for_hook, &pinned_root_for_hook).unwrap();
            fs::create_dir(&lease_root_for_hook).unwrap();
            fs::write(
                lease_root_for_hook.join("unrelated-sentinel"),
                b"must survive",
            )
            .unwrap();
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
    let replacement = lease_root.clone();
    #[cfg(not(target_os = "freebsd"))]
    let replacement = cleanup_quarantine(&root);
    assert_eq!(
        fs::read(replacement.join("unrelated-sentinel")).unwrap(),
        b"must survive"
    );

    fs::remove_dir_all(&replacement).unwrap();
    fs::rename(&pinned_root, &lease_root).unwrap();
    assert_eq!(
        scheduler.recover(&id).unwrap(),
        repaired_recovery_disposition()
    );
    fs::remove_dir_all(root).unwrap();
}
