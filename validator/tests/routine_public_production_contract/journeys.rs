use super::scenario::{
    ContainedContender, Fixture, contain_contender, pass_node, prefix_route, run_bounded_contender,
    tree,
};
use serde_json::Value;
use std::fs;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

#[test]
fn fixture_matrix_names_the_public_production_contract_without_claim_effect() {
    let value: Value = serde_json::from_slice(include_bytes!(
        "../../../fixtures/routine-public-production/cases.json"
    ))
    .unwrap();
    assert_eq!(value["schema_version"], "RoutinePublicProductionCases-v2");
    assert_eq!(value["supported_host"], "target_vendor=apple");
    assert_eq!(value["claim_effect"], "none");
    let cases = value["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 7);
    assert_eq!(
        cases
            .iter()
            .filter_map(Value::as_str)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        7
    );
    let runtime_cases = value["immutable_runtime_cases"].as_array().unwrap();
    assert_eq!(runtime_cases.len(), 8);
    assert_eq!(
        runtime_cases
            .iter()
            .filter_map(Value::as_str)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        8
    );
    assert!(
        value["installed_runtime_ceiling"]
            .as_str()
            .unwrap()
            .contains("immutable installed ultragoal runtime")
    );
}
#[test]
fn clean_public_routine_is_a_zero_effect_noop() {
    let mut fixture = Fixture::new(
        "clean-no-op",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        false,
        false,
    );
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let before_status = fixture.status();
    let output = fixture.run();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let value = Fixture::value(&output);
    assert_eq!(value["status"], "clean-no-op");
    assert_eq!(value["effect"], "none");
    let after_root = tree(&fixture.root);
    for (path, value) in before_root {
        assert_eq!(
            after_root.get(&path),
            Some(&value),
            "source changed: {path}"
        );
    }
    assert_eq!(tree(&fixture.home), before_home);
    assert_eq!(fixture.status(), before_status);
    fixture.teardown_after_assertions();
}
#[test]
fn dirty_public_effect_executes_once_then_repeat_refuses_without_mutation() {
    let mut fixture = Fixture::new(
        "execute-reuse",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let before_root = tree(&fixture.root);
    let executed = fixture.run();
    assert_eq!(executed.status.code(), Some(0), "{executed:?}");
    assert!(executed.stderr.is_empty(), "{executed:?}");
    let executed_value = Fixture::value(&executed);
    assert_eq!(executed_value["status"], "executed");
    assert_eq!(executed_value["nodes"][0]["disposition"], "executed");
    let after_root = tree(&fixture.root);
    for (path, value) in before_root {
        assert_eq!(
            after_root.get(&path),
            Some(&value),
            "source changed: {path}"
        );
    }

    let before_repeat = tree(&fixture.root);
    assert_session_continuity_refusal(&fixture.run());
    assert_eq!(tree(&fixture.root), before_repeat);
    fixture.teardown_after_assertions();
}
#[test]
fn authorized_fresh_execution_then_exact_repeat_requires_session_continuity() {
    let mut fixture = Fixture::new(
        "authorized-fresh-repeat",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    assert!(!fixture.root.join("target").exists());

    let executed = fixture.run();
    assert_eq!(executed.status.code(), Some(0), "{executed:?}");
    assert!(executed.stderr.is_empty(), "{executed:?}");
    let executed = Fixture::value(&executed);
    assert_eq!(executed["status"], "executed");
    assert_eq!(executed["nodes"][0]["disposition"], "executed");
    let scope = fixture.root.join("target/routine/compile");
    assert!(scope.is_dir());
    assert_eq!(fs::read_dir(&scope).unwrap().count(), 0);

    assert_session_continuity_refusal(&fixture.run());
    assert_eq!(fs::read_dir(scope).unwrap().count(), 0);
    fixture.teardown_after_assertions();
}

#[test]
fn public_output_creation_is_observed_only_after_the_durable_journal() {
    let mut fixture = Fixture::new(
        "durable-output-order",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let scope = fixture.root.join("target/routine/compile");
    let state = fixture.authority_root().join("routine-authority.state");
    let stopped = Arc::new(AtomicBool::new(false));
    let watcher_stopped = Arc::clone(&stopped);
    let watcher = std::thread::spawn(move || {
        loop {
            if scope.is_dir() {
                let durable =
                    fs::read(&state).map_err(|_| "output appeared before durable state")?;
                let text = String::from_utf8(durable).map_err(|_| "durable state is not UTF-8")?;
                if !text.contains("\"output_journal\"") || !text.contains("target/routine/compile")
                {
                    return Err("output appeared before its durable journal binding");
                }
                return Ok(());
            }
            if watcher_stopped.load(Ordering::Acquire) {
                return Err("child exited without provisioning output");
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    });
    let mut command = fixture.base_command();
    command.args(["--json", "check", "routine"]);
    let observed = contain_contender(
        run_bounded_contender(&mut command, Duration::from_secs(60)),
        Duration::from_secs(2),
    );
    stopped.store(true, Ordering::Release);
    let journal = watcher.join().expect("journal watcher panicked");
    let outcome = match observed {
        ContainedContender::Exited(output) => Ok(output),
        ContainedContender::TerminatedAndReaped(_) => Err("public command timed out"),
        ContainedContender::ExitedAtDeadline(_) => Err("public command exited after deadline"),
        ContainedContender::ReapedWithFailure(_) => Err("public command cleanup failed"),
    };
    fixture.teardown_after_assertions();
    assert_eq!(journal, Ok(()));
    let output = outcome.unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
}

#[test]
fn terminal_failure_serializes_no_recovery_and_fresh_process_refuses_takeover() {
    let mut fixture = Fixture::new(
        "terminal-failure-retry",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    fs::write(fixture.root.join("src/lib.rs"), b"pub fn broken(\n").unwrap();

    let output = fixture.run();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let value = Fixture::value(&output);
    assert_eq!(value["status"], "incomplete");
    assert_eq!(value["recovery_required"], false);
    assert_eq!(value["nodes"][0]["disposition"], "failed");
    assert_session_continuity_refusal(&fixture.run());
    fixture.teardown_after_assertions();
}

#[test]
fn independent_fresh_authority_roots_execute_once_and_refuse_repeat() {
    let mut first = Fixture::new(
        "binding-first",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let mut second = Fixture::new(
        "binding-second",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    assert_status(&first, "executed");
    assert_status(&second, "executed");
    assert_session_continuity_refusal(&first.run());
    assert_session_continuity_refusal(&second.run());
    second.teardown_after_assertions();
    first.teardown_after_assertions();
}

fn assert_session_continuity_refusal(output: &std::process::Output) {
    assert_eq!(output.status.code(), Some(3), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let value: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(
        value["cause"],
        "routine-production-session-continuity-required"
    );
    let ceiling = value["resulting_ceiling"].as_str().unwrap();
    assert!(ceiling.contains("reuse, repeat execution"), "{ceiling}");
    assert!(ceiling.contains("external monotonic head"), "{ceiling}");
}

fn assert_status(fixture: &Fixture, expected: &str) {
    let output = fixture.run();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    assert_eq!(Fixture::value(&output)["status"], expected);
}
