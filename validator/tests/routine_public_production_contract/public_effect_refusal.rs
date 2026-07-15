use super::scenario::{
    BoundedContender, Fixture, pass_node, prefix_route, run_bounded_contender, tree,
};
use serde_json::Value;
use std::fs;
use std::fs::OpenOptions;
use std::os::fd::AsRawFd;
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Output};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

const LOCK_CONTENTION_BOUND: Duration = Duration::from_secs(10);

fn dirty_fixture(label: &str, provision_host: bool) -> Fixture {
    Fixture::new(
        label,
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        provision_host,
    )
}

fn assert_public_refusal(output: &Output) {
    assert_ne!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let diagnostic: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(
        diagnostic["diagnostic_id"],
        "successor_runtime_authority_required"
    );
    assert_eq!(diagnostic["effect"], "none");
    let ceiling = diagnostic["resulting_ceiling"].as_str().unwrap();
    assert!(
        ceiling.contains("source-local"),
        "unexpected ceiling: {ceiling}"
    );
    assert!(
        !ceiling.contains("local issuer"),
        "stale local issuer claim: {ceiling}"
    );
}

fn assert_public_busy(output: &Output) {
    assert_public_refusal(output);
    let diagnostic: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(
        diagnostic["cause"],
        "the exact routine host authority is busy with another active public invocation"
    );
}

fn assert_fixture_unchanged(
    fixture: &Fixture,
    root: &std::collections::BTreeMap<String, String>,
    home: &std::collections::BTreeMap<String, String>,
    status: &[u8],
) {
    assert_eq!(tree(&fixture.root), *root);
    assert_eq!(tree(&fixture.home), *home);
    assert_eq!(fixture.status(), status);
    assert!(!fixture.root.join("target").exists());
}

#[test]
fn valid_host_runs_through_local_issuer_before_repeat() {
    let mut fixture = dirty_fixture("local-issuer-first-run", true);
    let output = fixture.run();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    assert_eq!(Fixture::value(&output)["status"], "executed");
    assert!(fixture.authority_root().is_dir());
    assert!(fixture.root.join("target/routine/compile").is_dir());
    fixture.teardown_after_assertions();
}

#[test]
fn missing_local_host_repeat_refusals_never_initialize_state_or_outputs() {
    let mut fixture = dirty_fixture("missing-local-host-repeat", false);
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let before_status = fixture.status();
    assert!(!fixture.state_root().exists());

    for _ in 0..4 {
        assert_public_refusal(&fixture.run());
        assert_fixture_unchanged(&fixture, &before_root, &before_home, &before_status);
        assert!(!fixture.state_root().exists());
    }
    fixture.teardown_after_assertions();
}

#[test]
fn concurrent_local_issuer_attempts_have_no_forged_success() {
    let mut fixture = dirty_fixture("local-issuer-concurrent", true);

    std::thread::scope(|scope| {
        let done = Arc::new(AtomicBool::new(false));
        let monitor_done = Arc::clone(&done);
        let monitored = &fixture;
        let monitor = scope.spawn(move || {
            while !monitor_done.load(Ordering::Acquire) {
                assert!(monitored.authority_root().exists());
                std::thread::yield_now();
            }
        });
        let attempts = (0..8)
            .map(|_| scope.spawn(|| fixture.run()))
            .collect::<Vec<_>>();
        let outputs = attempts
            .into_iter()
            .map(|attempt| attempt.join().unwrap())
            .collect::<Vec<_>>();
        assert!(
            outputs.iter().any(|output| output.status.success()),
            "no local issuer attempt completed: {outputs:?}"
        );
        for output in outputs {
            if !output.status.success() {
                assert_public_refusal(&output);
            }
        }
        done.store(true, Ordering::Release);
        monitor.join().unwrap();
    });

    assert!(fixture.authority_root().is_dir());
    assert!(fixture.root.join("target/routine/compile").is_dir());
    fixture.teardown_after_assertions();
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
    assert_eq!(
        unsafe { libc::flock(holder.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
        0
    );

    let mut command = fixture.command();
    let contender = match run_bounded_contender(&mut command, LOCK_CONTENTION_BOUND) {
        BoundedContender::Exited(output) => output,
        BoundedContender::TimedOut { output, kill_error } => {
            assert_eq!(unsafe { libc::flock(holder.as_raw_fd(), libc::LOCK_UN) }, 0);
            drop(holder);
            fixture.teardown_after_assertions();
            panic!(
                "public contender exceeded {LOCK_CONTENTION_BOUND:?}: {kill_error:?}; {output:?}"
            );
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
    match run_bounded_contender(&mut blocking, Duration::from_millis(100)) {
        BoundedContender::Exited(output) => panic!("blocking contender exited: {output:?}"),
        BoundedContender::TimedOut { output, kill_error } => {
            panic!("bounded contender timed out after reaping: {kill_error:?}; {output:?}")
        }
    }
}

#[test]
fn tool_path_substitution_refuses_before_state_or_spawn() {
    let mut fixture = dirty_fixture("tool-path-substitution", true);
    let probe_dir = fixture.container.join("process-probes");
    let marker = fixture.container.join("process-spawned");
    fs::create_dir(&probe_dir).unwrap();
    for name in ["git", "ultragoal", "cargo", "rustc"] {
        let probe = probe_dir.join(name);
        fs::write(
            &probe,
            format!(
                "#!/bin/sh\n/usr/bin/touch '{}'\nexit 90\n",
                marker.display()
            ),
        )
        .unwrap();
        fs::set_permissions(&probe, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let before_status = fixture.status();
    assert!(!marker.exists(), "fixture setup unexpectedly ran a probe");
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);

    let mut command = fixture.base_command();
    let output = command
        .env("PATH", &probe_dir)
        .args(["--json", "check", "routine"])
        .output()
        .unwrap();

    assert_public_refusal(&output);
    assert!(
        !marker.exists(),
        "public refusal spawned a substituted tool"
    );
    assert_fixture_unchanged(&fixture, &before_root, &before_home, &before_status);
    assert_eq!(fs::read_dir(fixture.authority_root()).unwrap().count(), 0);
    fixture.teardown_after_assertions();
}
