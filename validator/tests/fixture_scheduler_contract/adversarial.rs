use crate::fixture_scheduler::*;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

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
