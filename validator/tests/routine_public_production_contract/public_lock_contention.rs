mod controls;
mod supervisor;

use self::supervisor::{
    SupervisorOutcome, SupervisorPlan, assert_fixture_lock_released, finish_fixture,
    run_child_if_requested, run_supervisor,
};
use super::public_effect_refusal::{assert_public_refusal, dirty_fixture};
use super::scenario::tree;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::time::Duration;

const LOCK_CONTENTION_BOUND: Duration = Duration::from_secs(20);
const TEST_NAME: &str = "routine_public_production_contract::public_lock_contention::\
    held_public_lock_refuses_a_real_contender_without_effect_then_allows_retry";

fn assert_public_busy(output: &std::process::Output) {
    assert_public_refusal(output);
    let diagnostic: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(
        diagnostic["cause"],
        "the exact routine host authority is busy with another active public invocation"
    );
}

#[test]
fn held_public_lock_refuses_a_real_contender_without_effect_then_allows_retry() {
    if run_child_if_requested() {
        return;
    }
    let mut fixture = dirty_fixture("held-lock-contender", true);
    let before_root = tree(&fixture.root);
    let before_status = fixture.status();
    let observed = run_supervisor(
        TEST_NAME,
        &fixture,
        SupervisorPlan {
            case: "real-public-lock",
            execution_bound: LOCK_CONTENTION_BOUND,
            cleanup_bound: Duration::from_secs(2),
            primary_kill_refusals: 0,
            group_signal_refusals: 0,
            reap_status_refusals: 0,
            pipe_drain_refusals: 0,
        },
    );

    finish_fixture(&mut fixture, |fixture| {
        assert!(observed.faults.group_is_absent());
        assert_fixture_lock_released(fixture);
        let output = match &observed.outcome {
            SupervisorOutcome::Exited(output) => output,
            other => panic!("public contention supervisor did not exit normally: {other:?}"),
        };
        assert!(output.status.success(), "{output:?}");
        assert_eq!(fixture.status(), before_status);
        assert!(fixture.authority_root().is_dir());
        let output_dir = fixture.root.join("target/routine/compile");
        assert!(output_dir.is_dir());
        assert_eq!(fs::read_dir(&output_dir).unwrap().count(), 0);
        assert_eq!(
            new_root_paths(fixture, &before_root),
            expected_output_paths()
        );
        assert!(
            tree(&fixture.home)
                .keys()
                .all(|path| !path.contains("launch-")),
            "supervisor left staged launch custody"
        );
    });
}

fn new_root_paths(
    fixture: &super::scenario::Fixture,
    before: &std::collections::BTreeMap<String, String>,
) -> BTreeSet<String> {
    tree(&fixture.root)
        .into_keys()
        .filter(|path| !before.contains_key(path))
        .collect()
}

fn expected_output_paths() -> BTreeSet<String> {
    ["target", "target/routine", "target/routine/compile"]
        .into_iter()
        .map(str::to_owned)
        .collect()
}
