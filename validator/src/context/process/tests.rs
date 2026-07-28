use super::*;
use std::time::Instant;

#[test]
fn descendant_after_leader_exit_is_not_signaled_by_reused_group_identity() {
    let started = Instant::now();
    let result = run_bounded_allow_failure(
        Path::new("/bin/sh"),
        &[OsString::from("-c"), OsString::from("(sleep 10) & exit 0")],
        Path::new("/"),
        Duration::from_secs(2),
    );

    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(matches!(
        result,
        Err(ContextError::Probe { message, .. })
            if message.contains("custody became ambiguous")
                && message.contains("reconciliation required")
    ));
}

#[test]
fn stdout_and_stderr_are_drained_concurrently() {
    let result = run_bounded_allow_failure(
        Path::new("/bin/sh"),
        &[
            OsString::from("-c"),
            OsString::from(
                "i=0; while [ \"$i\" -lt 4096 ]; do printf out; printf err >&2; i=$((i + 1)); done",
            ),
        ],
        Path::new("/"),
        Duration::from_secs(2),
    )
    .unwrap();

    assert!(result.success);
    assert_eq!(result.stdout.len(), 4096 * 3);
    assert_eq!(result.stderr.len(), 4096 * 3);
}

#[test]
fn deadline_kills_and_reaps_the_process_group() {
    let started = Instant::now();
    let result = run_bounded_allow_failure(
        Path::new("/bin/sh"),
        &[OsString::from("-c"), OsString::from("sleep 10")],
        Path::new("/"),
        Duration::from_millis(25),
    );

    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(matches!(
        result,
        Err(ContextError::Probe { message, .. }) if message.contains("timed out after 25 ms")
    ));
}
