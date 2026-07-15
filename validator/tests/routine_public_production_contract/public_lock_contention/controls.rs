use super::super::public_effect_refusal::{assert_fixture_unchanged, dirty_fixture};
use super::super::scenario::tree;
use super::supervisor::{
    SupervisorObservation, SupervisorOutcome, SupervisorPlan, assert_fixture_lock_released,
    finish_fixture, run_child_if_requested, run_supervisor,
};
use std::time::Duration;

const TEST_NAME: &str = "routine_public_production_contract::public_lock_contention::controls::\
    supervisor_faults_leave_no_live_group_or_fixture_residue";

#[test]
fn supervisor_faults_leave_no_live_group_or_fixture_residue() {
    if run_child_if_requested() {
        return;
    }
    run_case("hang", plan("hang"), |observed| {
        assert_terminated(observed);
    });
    run_case("panic-live", plan("panic-live-child"), |observed| {
        let output = reaped_output(observed);
        assert!(!output.status.success(), "{output:?}");
    });
    run_case(
        "term-resistant-descendant",
        plan("ignore-term-descendant"),
        assert_terminated,
    );
    run_case(
        "primary-kill-refusal",
        SupervisorPlan {
            primary_kill_refusals: 1,
            ..plan("hang")
        },
        |observed| {
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
        |observed| {
            assert_terminated(observed);
            assert_eq!(observed.faults.reap_status_refusals(), 40);
        },
    );
    run_case("pipe-pressure", plan("pipe-pressure"), |observed| {
        let output = terminated_output(observed);
        assert_eq!(output.stdout.len(), 64 * 1024);
    });
    run_case(
        "pipe-refusal",
        SupervisorPlan {
            pipe_drain_refusals: 1,
            ..plan("hang")
        },
        |observed| {
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
        |observed| match &observed.outcome {
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
        |observed| match &observed.outcome {
            SupervisorOutcome::Exited(output)
            | SupervisorOutcome::ExitedAtDeadline(output)
            | SupervisorOutcome::TerminatedAndReaped(output) => {
                assert!(output.status.success() || output.status.code().is_none());
            }
            other => panic!("exit/deadline supervisor was not reaped: {other:?}"),
        },
    );
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
    }
}

fn run_case(
    label: &str,
    plan: SupervisorPlan,
    inspect: impl FnOnce(&SupervisorObservation) + std::panic::UnwindSafe,
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
        inspect(&observed);
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
