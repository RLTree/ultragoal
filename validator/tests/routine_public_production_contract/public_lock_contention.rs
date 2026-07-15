use super::public_effect_refusal::{
    assert_fixture_unchanged, assert_public_refusal, dirty_fixture,
};
use super::scenario::{BoundedContender, ContenderCustody, Fixture, run_bounded_contender, tree};
use serde_json::Value;
use std::fs;
use std::fs::OpenOptions;
use std::os::fd::AsRawFd;
use std::process::Command;
use std::time::{Duration, Instant};

const LOCK_CONTENTION_BOUND: Duration = Duration::from_secs(20);

fn assert_public_busy(output: &std::process::Output) {
    assert_public_refusal(output);
    let diagnostic: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(
        diagnostic["cause"],
        "the exact routine host authority is busy with another active public invocation"
    );
}

fn retain_unresolved_contender(
    holder: std::fs::File,
    custody: ContenderCustody,
    fixture: &Fixture,
) -> ! {
    let child = custody.child_id();
    let cause = custody.cause();
    let retained_holder = Box::leak(Box::new(holder));
    let retained_custody = Box::leak(Box::new(custody));
    panic!(
        "public contender custody remains unresolved; holder fd {} and child {} are retained against fixture {}: {} ({retained_custody:?})",
        retained_holder.as_raw_fd(),
        child,
        fixture.container.display(),
        cause,
    )
}

fn retain_unresolved_child(custody: ContenderCustody, context: &str) -> ! {
    let child = custody.child_id();
    let cause = custody.cause();
    let retained_custody = Box::leak(Box::new(custody));
    panic!("{context} retained unresolved child {child}: {cause} ({retained_custody:?})")
}

#[test]
fn held_public_lock_refuses_a_real_contender_without_effect_then_allows_retry() {
    let mut fixture = dirty_fixture("held-lock-contender", true);
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let before_status = fixture.status();
    let holder = OpenOptions::new()
        .read(true)
        .write(true)
        .open(fixture.lock_path())
        .unwrap();
    assert_ne!(
        unsafe { libc::fcntl(holder.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC,
        0
    );
    assert_eq!(
        unsafe { libc::flock(holder.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
        0
    );

    let mut command = fixture.command();
    let observed = run_bounded_contender(&mut command, LOCK_CONTENTION_BOUND);
    let observed = match observed {
        BoundedContender::Unresolved(custody) => custody.escalate(Duration::from_secs(2)),
        reaped => reaped,
    };
    let contender = match observed {
        BoundedContender::Exited(output) => output,
        BoundedContender::Unresolved(custody) => {
            retain_unresolved_contender(holder, custody, &fixture)
        }
        other => {
            assert_eq!(unsafe { libc::flock(holder.as_raw_fd(), libc::LOCK_UN) }, 0);
            drop(holder);
            fixture.teardown_after_assertions();
            panic!("public contender did not finish before {LOCK_CONTENTION_BOUND:?}: {other:?}");
        }
    };
    assert_public_busy(&contender);
    assert_fixture_unchanged(&fixture, &before_root, &before_home, &before_status);
    assert_eq!(fs::read_dir(fixture.authority_root()).unwrap().count(), 0);

    assert_eq!(unsafe { libc::flock(holder.as_raw_fd(), libc::LOCK_UN) }, 0);
    drop(holder);
    let retry = fixture.run();
    assert_eq!(retry.status.code(), Some(0), "{retry:?}");
    assert!(fixture.root.join("target/routine/compile").is_dir());
    fixture.teardown_after_assertions();
}

#[test]
#[should_panic(expected = "bounded contender timed out")]
fn blocking_contender_is_reaped_and_never_accepted_as_a_public_result() {
    let mut blocking = Command::new("/bin/sleep");
    blocking.arg("60");
    let started = Instant::now();
    let observed = run_bounded_contender(&mut blocking, Duration::from_millis(100));
    let observed = match observed {
        BoundedContender::Unresolved(custody) => custody.escalate(Duration::from_secs(1)),
        reaped => reaped,
    };
    match observed {
        BoundedContender::TerminatedAndReaped(output) => {
            assert!(started.elapsed() < Duration::from_secs(1), "{output:?}");
            panic!("bounded contender timed out after verified reap: {output:?}")
        }
        BoundedContender::Unresolved(custody) => {
            retain_unresolved_child(custody, "blocking contender cleanup")
        }
        other => panic!("blocking contender was not verified reaped: {other:?}"),
    }
}
