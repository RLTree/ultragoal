use super::super::filesystem::{PinnedExecutable, RootAnchor};
use super::super::outcome::RoutineCancellation;
use super::process_termination_tests::ProcessFixture;
use super::*;
use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

#[test]
fn every_setup_running_and_join_refusal_explicitly_reaps_custody() {
    let points = [
        ProcessFailurePoint::ProcessGroup,
        ProcessFailurePoint::StdoutNonblocking,
        ProcessFailurePoint::StderrNonblocking,
        ProcessFailurePoint::StdoutReaderStart,
        ProcessFailurePoint::StderrReaderStart,
        ProcessFailurePoint::StdinWriterStart,
        ProcessFailurePoint::Resume,
        ProcessFailurePoint::Wait,
        ProcessFailurePoint::StdoutJoin,
        ProcessFailurePoint::StderrJoin,
        ProcessFailurePoint::StdinJoin,
    ];
    for point in points {
        let fixture = ProcessFixture::new(&format!("explicit-{point:?}"));
        let observed = Arc::new(AtomicU64::new(0));
        let hook_observed = Arc::clone(&observed);
        set_test_process_failure(point, move || {
            hook_observed.fetch_add(1, Ordering::SeqCst);
        });
        let script = if matches!(
            point,
            ProcessFailurePoint::StdoutJoin
                | ProcessFailurePoint::StderrJoin
                | ProcessFailurePoint::StdinJoin
        ) {
            "exit 0"
        } else {
            "while :; do :; done"
        };
        let result = fixture.run_result(
            script,
            Duration::from_secs(2),
            1024,
            &RoutineCancellation::new(),
            || Ok(()),
        );
        assert!(result.is_err(), "{point:?} unexpectedly succeeded");
        assert_eq!(observed.load(Ordering::SeqCst), 1, "{point:?}");
        assert_last_group_absent();
        fixture.teardown();
    }
}

#[test]
fn started_refusal_and_cleanup_failure_are_both_explicit() {
    let fixture = ProcessFixture::new("started-refusal");
    let result = fixture.run_result(
        "while :; do :; done",
        Duration::from_secs(2),
        1024,
        &RoutineCancellation::new(),
        || Err(mediator_error("mediator-started-transition-injected")),
    );
    assert_eq!(error_cause(result), "mediator-started-transition-injected");
    assert_last_group_absent();
    fixture.teardown();

    let fixture = ProcessFixture::new("cleanup-refusal");
    set_test_process_failure(ProcessFailurePoint::Cleanup, || {});
    let result = fixture.run_result(
        "while :; do :; done",
        Duration::from_secs(2),
        1024,
        &RoutineCancellation::new(),
        || Err(mediator_error("mediator-started-transition-injected")),
    );
    assert_eq!(error_cause(result), "mediator-process-cleanup-injected");
    assert_last_group_absent();
    fixture.teardown();
}

#[test]
fn running_panic_preserves_payload_after_explicit_reap() {
    let fixture = ProcessFixture::new("panic");
    set_test_process_failure(ProcessFailurePoint::Wait, || {
        std::panic::panic_any("process-custody-panic")
    });
    let panic = match catch_unwind(AssertUnwindSafe(|| {
        fixture.run_result(
            "while :; do :; done",
            Duration::from_secs(2),
            1024,
            &RoutineCancellation::new(),
            || Ok(()),
        )
    })) {
        Err(payload) => payload,
        Ok(_) => panic!("injected running panic was swallowed"),
    };
    assert_eq!(panic.downcast_ref::<&str>(), Some(&"process-custody-panic"));
    assert_last_group_absent();
    fixture.teardown();
}

#[test]
fn panic_cleanup_failure_is_typed_before_authority_transition() {
    for cleanup_panics in [false, true] {
        let fixture = ProcessFixture::new(if cleanup_panics {
            "panic-cleanup-panic"
        } else {
            "panic-cleanup-error"
        });
        set_test_process_failure(ProcessFailurePoint::Wait, || {
            std::panic::panic_any("process-custody-original-panic")
        });
        append_test_process_failure(ProcessFailurePoint::Cleanup, move || {
            if cleanup_panics {
                std::panic::panic_any("process-custody-cleanup-panic");
            }
        });
        let payload = catch_process_panic(&fixture);
        let (original, cleanup_failure) = take_process_custody_panic(payload)
            .unwrap_or_else(|_| panic!("cleanup failure lacked typed custody"));
        assert_eq!(
            original.downcast_ref::<&str>(),
            Some(&"process-custody-original-panic")
        );
        if cleanup_panics {
            assert_eq!(
                cleanup_failure.downcast_ref::<&str>(),
                Some(&"process-custody-cleanup-panic")
            );
        } else {
            let error = cleanup_failure
                .downcast_ref::<crate::routine_work::RoutineError>()
                .unwrap_or_else(|| panic!("cleanup error changed type"));
            assert_eq!(error.cause(), "mediator-process-cleanup-injected");
        }
        assert_last_group_absent();
        fixture.teardown();
    }
}

fn catch_process_panic(fixture: &ProcessFixture) -> Box<dyn std::any::Any + Send> {
    match catch_unwind(AssertUnwindSafe(|| {
        fixture.run_result(
            "while :; do :; done",
            Duration::from_secs(2),
            1024,
            &RoutineCancellation::new(),
            || Ok(()),
        )
    })) {
        Err(payload) => payload,
        Ok(_) => panic!("cleanup failure did not preserve the initiating panic"),
    }
}

#[test]
fn dropping_setup_or_running_custody_does_not_signal_or_reap() {
    let fixture = ProcessFixture::new("drop-inert");
    let setup = suspended_setup(&fixture);
    let group = setup.process_group().unwrap();
    drop(setup);
    assert!(process_group_exists(group).unwrap());
    reap_raw_test_child(group);

    let setup = suspended_setup(&fixture);
    let group = setup.process_group().unwrap();
    let running = setup.into_running(group);
    drop(running);
    assert!(process_group_exists(group).unwrap());
    reap_raw_test_child(group);
    fixture.teardown();
}

fn suspended_setup(fixture: &ProcessFixture) -> SpawnSetupGuard {
    let root = RootAnchor::open(&fixture.workspace).unwrap();
    let program = PinnedExecutable::open_unbound(std::path::Path::new("/bin/sh")).unwrap();
    let environment = BTreeMap::from([(
        crate::routine_work::CHILD_MODE_ENV.to_owned(),
        crate::routine_work::CHILD_MODE_VALUE.to_owned(),
    )]);
    spawn_exact_program(
        &program,
        &root,
        &[
            "sh".to_owned(),
            "-c".to_owned(),
            "while :; do :; done".to_owned(),
        ],
        &environment,
    )
    .unwrap()
}

fn assert_last_group_absent() {
    let group = test_last_spawn_group().expect("spawned process group was not observed");
    assert!(
        !process_group_exists(group).unwrap(),
        "process group survived explicit cleanup"
    );
}

fn error_cause(
    result: Result<ProcessObservation, crate::routine_work::RoutineError>,
) -> &'static str {
    match result {
        Err(error) => error.cause(),
        Ok(_) => panic!("injected process failure unexpectedly succeeded"),
    }
}

fn reap_raw_test_child(group: ProcessGroupId) {
    signal_group(group, libc::SIGKILL).unwrap();
    let pid = -group.signal_target().unwrap();
    let mut status = 0;
    loop {
        let result = unsafe { libc::waitpid(pid, &mut status, 0) };
        if result == pid {
            break;
        }
        assert_eq!(
            std::io::Error::last_os_error().kind(),
            std::io::ErrorKind::Interrupted
        );
    }
    wait_group_absent(group).unwrap();
}
