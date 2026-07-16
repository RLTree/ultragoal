use super::super::public_effect_refusal::{assert_fixture_unchanged, dirty_fixture};
use super::super::scenario::tree;
use super::invocation_capability::CapabilityMode;
use super::supervisor::{
    SupervisorObservation, SupervisorOutcome, SupervisorPlan, assert_fixture_lock_released,
    finish_fixture, run_child_if_requested, run_supervisor,
};
use std::time::Duration;

const PROBE_TEST: &str = "routine_public_production_contract::public_lock_contention::selection_controls::\
    helper_capability_probe_reaches_parent_assertions_without_a_capability";

#[test]
fn helper_capability_probe_reaches_parent_assertions_without_a_capability() {
    if run_child_if_requested(PROBE_TEST) {
        return;
    }
    println!("routine-public-capability-v1: parent-assertion-reached");
}

#[test]
fn helper_selection_requires_one_parent_issued_capability() {
    let mut fixture = dirty_fixture("helper-selection", true);
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let before_status = fixture.status();
    for mode in [
        CapabilityMode::Omit,
        CapabilityMode::WrongFd,
        CapabilityMode::Malformed,
        CapabilityMode::Duplicate,
        CapabilityMode::Trailing,
        CapabilityMode::WrongParent,
        CapabilityMode::WrongTest,
        CapabilityMode::WrongCase,
        CapabilityMode::WrongRoot,
        CapabilityMode::WrongHome,
        CapabilityMode::WrongBinary,
    ] {
        let observed = run_supervisor(PROBE_TEST, &fixture, plan("capability-probe", mode));
        let output = reaped_output(&observed);
        if matches!(mode, CapabilityMode::Omit) {
            assert!(output.status.success(), "{output:?}");
            assert!(String::from_utf8_lossy(&output.stdout).contains("parent-assertion-reached"));
        } else {
            assert!(!output.status.success(), "{mode:?}: {output:?}");
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                stderr.contains("helper capability refused"),
                "{mode:?}: {stderr}"
            );
            assert!(!stderr.contains("helper-selected"));
        }
        assert!(observed.faults.group_is_absent());
    }
    let replay = run_supervisor(
        PROBE_TEST,
        &fixture,
        plan("capability-replay", CapabilityMode::Valid),
    );
    let output = reaped_output(&replay);
    assert!(output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("replay-refused"));
    finish_fixture(&mut fixture, |fixture| {
        assert_fixture_unchanged(fixture, &before_root, &before_home, &before_status);
        assert_fixture_lock_released(fixture);
    });
}

fn plan(case: &'static str, capability: CapabilityMode) -> SupervisorPlan {
    SupervisorPlan {
        case,
        execution_bound: Duration::from_secs(1),
        cleanup_bound: Duration::from_millis(100),
        primary_kill_refusals: 0,
        group_signal_refusals: 0,
        reap_status_refusals: 0,
        pipe_drain_refusals: 0,
        capability,
    }
}

fn reaped_output(observed: &SupervisorObservation) -> &std::process::Output {
    match &observed.outcome {
        SupervisorOutcome::Exited(output)
        | SupervisorOutcome::TerminatedAndReaped(output)
        | SupervisorOutcome::ExitedAtDeadline(output) => output,
        other => panic!("helper selection left unreaped custody: {other:?}"),
    }
}
