mod controls;
mod invocation_capability;
mod live_child;
mod selection_controls;
mod supervisor;

use self::invocation_capability::CapabilityMode;
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
    if run_child_if_requested(TEST_NAME) {
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
            capability: CapabilityMode::Valid,
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
        assert_terminal_event_after_retry(fixture);
        assert_eq!(
            new_root_paths(fixture, &before_root),
            expected_output_paths(fixture)
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

fn expected_output_paths(fixture: &super::scenario::Fixture) -> BTreeSet<String> {
    let mut paths = [
        "target",
        "target/routine",
        "target/routine/compile",
        "validation_artifacts",
        "validation_artifacts/observability",
        "validation_artifacts/observability/spool",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<BTreeSet<_>>();
    paths.insert(
        event_path(fixture)
            .strip_prefix(&fixture.root)
            .unwrap()
            .to_string_lossy()
            .into_owned(),
    );
    paths
}

fn assert_terminal_event_after_retry(fixture: &super::scenario::Fixture) {
    let event_path = event_path(fixture);
    let rows = fs::read_to_string(&event_path)
        .unwrap()
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    assert_eq!(
        rows.len(),
        1,
        "successful retry must append one terminal event"
    );
    let row: Value = serde_json::from_str(&rows[0]).unwrap();
    let event = &row["event"];
    let checkpoint: Value =
        serde_json::from_slice(&fs::read(fixture.checkpoint_path()).unwrap()).unwrap();
    assert_eq!(event["event_id"], checkpoint["event_id"]);
    assert_eq!(event["sequence"], checkpoint["event_sequence"]);
    assert_eq!(
        event["observed_at_unix_ms"],
        checkpoint["event_observed_at_unix_ms"]
    );
    assert_eq!(event["operation"], "check.routine.terminal");
    assert_eq!(event["outcome"], checkpoint["event_status"]);
    assert_eq!(
        event["public_attributes"]["continuation_id"],
        checkpoint["continuation"]
    );
    assert_eq!(
        event["public_attributes"]["terminal_ledger_head"],
        checkpoint["authenticated_ledger_head"]
    );
    assert_eq!(
        event["public_attributes"]["routine_transition"],
        checkpoint["event_transition"]
    );
    assert_eq!(checkpoint["state"], "terminal-event-joined");
}

fn event_path(fixture: &super::scenario::Fixture) -> std::path::PathBuf {
    let spool = fixture
        .root
        .join("validation_artifacts/observability/spool");
    let paths = fs::read_dir(spool)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.starts_with("successor-events-") && name.ends_with(".jsonl")
                })
        })
        .collect::<Vec<_>>();
    assert_eq!(paths.len(), 1);
    paths.into_iter().next().unwrap()
}
