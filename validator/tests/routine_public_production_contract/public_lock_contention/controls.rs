use super::super::public_effect_refusal::{assert_fixture_unchanged, dirty_fixture};
use super::super::scenario::tree;
use super::invocation_capability::CapabilityMode;
use super::live_child::verify_live_child_panic;
use super::supervisor::{
    SupervisorObservation, SupervisorOutcome, SupervisorPlan, assert_fixture_lock_released,
    finish_fixture, run_child_if_requested, run_supervisor,
};
use std::time::Duration;

const TEST_NAME: &str = "routine_public_production_contract::public_lock_contention::controls::\
    supervisor_faults_leave_no_live_group_or_fixture_residue";
const LIVE_CHILD_TEST_NAME: &str = "routine_public_production_contract::public_lock_contention::controls::\
    live_child_handshake_rejects_noncausal_panic_variants";

#[test]
fn supervisor_faults_leave_no_live_group_or_fixture_residue() {
    if run_child_if_requested(TEST_NAME) {
        return;
    }
    run_case("hang", plan("hang"), |_, observed| {
        assert_terminated(observed);
    });
    run_case(
        "panic-live",
        handshake_plan("panic-live-child"),
        |fixture, observed| {
            let output = reaped_output(observed);
            assert!(!output.status.success(), "{output:?}");
            verify_live_child_panic(output, fixture, observed.faults.process_group())
                .unwrap_or_else(|error| panic!("{error}: {output:?}"));
        },
    );
    run_case(
        "term-resistant-descendant",
        plan("ignore-term-descendant"),
        |_, observed| assert_terminated(observed),
    );
    run_case(
        "primary-kill-refusal",
        SupervisorPlan {
            primary_kill_refusals: 1,
            ..plan("hang")
        },
        |_, observed| {
            assert_terminated(observed);
            assert_eq!(observed.faults.primary_kill_refusals(), 1);
        },
    );
    run_case(
        "late-reap",
        SupervisorPlan {
            cleanup_bound: Duration::from_millis(30),
            reap_status_refusals: 40,
            ..plan("hang")
        },
        |_, observed| {
            assert_terminated(observed);
            assert_eq!(observed.faults.reap_status_refusals(), 40);
        },
    );
    run_case(
        "pipe-pressure",
        SupervisorPlan {
            execution_bound: Duration::from_secs(1),
            ..plan("pipe-pressure")
        },
        |_, observed| {
            let output = terminated_output(observed);
            assert_eq!(output.stdout.len(), 64 * 1024);
        },
    );
    run_case(
        "pipe-refusal",
        SupervisorPlan {
            pipe_drain_refusals: 1,
            ..plan("hang")
        },
        |_, observed| {
            assert!(matches!(
                observed.outcome,
                SupervisorOutcome::ReapedWithFailure("contender-pipe-drain-injected-refusal")
            ));
            assert_eq!(observed.faults.pipe_drain_refusals(), 1);
        },
    );
    run_case(
        "normal-exit",
        SupervisorPlan {
            execution_bound: Duration::from_secs(1),
            ..plan("exit")
        },
        |_, observed| match &observed.outcome {
            SupervisorOutcome::Exited(output) => assert!(output.status.success(), "{output:?}"),
            other => panic!("normal supervisor did not exit normally: {other:?}"),
        },
    );
    run_case(
        "exit-deadline-race",
        SupervisorPlan {
            execution_bound: Duration::ZERO,
            ..plan("exit")
        },
        |_, observed| match &observed.outcome {
            SupervisorOutcome::Exited(output)
            | SupervisorOutcome::ExitedAtDeadline(output)
            | SupervisorOutcome::TerminatedAndReaped(output) => {
                assert!(output.status.success() || output.status.code().is_none());
            }
            other => panic!("exit/deadline supervisor was not reaped: {other:?}"),
        },
    );
}

#[test]
fn live_child_handshake_rejects_noncausal_panic_variants() {
    if run_child_if_requested(LIVE_CHILD_TEST_NAME) {
        return;
    }
    for (case, diagnostic) in [
        (
            "panic-live-child-spawn-refusal",
            "live-child-handshake-v1 spawn-refusal for wrong configured binary",
        ),
        (
            "panic-live-child-early-exit",
            "live-child-handshake-v1 early-exit",
        ),
        (
            "panic-live-child-signal-failure",
            "live-child-handshake-v1 signal-failure",
        ),
        (
            "panic-live-child-wrong-group",
            "live-child-handshake-v1 wrong-group",
        ),
        (
            "panic-live-child-missing-handshake",
            "routine-public-live-child-v1: verified configured product child is stopped",
        ),
        (
            "panic-live-child-duplicate-handshake",
            "routine-public-live-child-v1: verified configured product child is stopped",
        ),
        (
            "panic-live-child-malformed-handshake",
            "routine-public-live-child-v1: verified configured product child is stopped",
        ),
        (
            "panic-live-child-wrong-binary",
            "routine-public-live-child-v1: verified configured product child is stopped",
        ),
        (
            "panic-live-child-wrong-sentinel",
            "live-child-handshake-v1 wrong panic sentinel",
        ),
    ] {
        run_case(case, handshake_plan(case), |fixture, observed| {
            let output = reaped_output(observed);
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(stderr.contains(diagnostic), "{case}: {output:?}");
            let verification =
                verify_live_child_panic(output, fixture, observed.faults.process_group());
            assert!(
                verification.is_err(),
                "{case} was credited as the verified live-child panic: {output:?}"
            );
        });
    }
}

fn plan(case: &'static str) -> SupervisorPlan {
    SupervisorPlan {
        case,
        execution_bound: Duration::from_millis(50),
        cleanup_bound: Duration::from_millis(100),
        primary_kill_refusals: 0,
        group_signal_refusals: 0,
        reap_status_refusals: 0,
        pipe_drain_refusals: 0,
        capability: CapabilityMode::Valid,
    }
}

fn handshake_plan(case: &'static str) -> SupervisorPlan {
    SupervisorPlan {
        execution_bound: Duration::from_secs(2),
        ..plan(case)
    }
}

fn run_case(
    label: &str,
    plan: SupervisorPlan,
    inspect: impl FnOnce(&super::super::scenario::Fixture, &SupervisorObservation)
    + std::panic::UnwindSafe,
) {
    let mut fixture = dirty_fixture(&format!("supervisor-{label}"), true);
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let before_status = fixture.status();
    let observed = run_supervisor(TEST_NAME, &fixture, plan);
    finish_fixture(&mut fixture, |fixture| {
        assert!(observed.elapsed < Duration::from_secs(3));
        assert!(observed.faults.group_is_absent());
        assert_fixture_unchanged(fixture, &before_root, &before_home, &before_status);
        assert_fixture_lock_released(fixture);
        inspect(fixture, &observed);
    });
}

fn assert_terminated(observed: &SupervisorObservation) {
    assert!(
        matches!(observed.outcome, SupervisorOutcome::TerminatedAndReaped(_)),
        "supervisor did not terminate inside the bound: {:?}",
        observed.outcome
    );
}

fn terminated_output(observed: &SupervisorObservation) -> &std::process::Output {
    match &observed.outcome {
        SupervisorOutcome::TerminatedAndReaped(output) => output,
        other => panic!("supervisor group was not terminated and reaped: {other:?}"),
    }
}

fn reaped_output(observed: &SupervisorObservation) -> &std::process::Output {
    match &observed.outcome {
        SupervisorOutcome::Exited(output)
        | SupervisorOutcome::TerminatedAndReaped(output)
        | SupervisorOutcome::ExitedAtDeadline(output) => output,
        other => panic!("supervisor did not produce a reaped output: {other:?}"),
    }
}
