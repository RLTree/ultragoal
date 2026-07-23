use super::scenario::{
    ContainedContender, Fixture, contain_contender, git, pass_node, prefix_route, routine_command,
    run_bounded_contender, tree,
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
fn dirty_public_effect_executes_once_then_exact_repeat_reuses_without_mutation() {
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
    assert_reused_without_effect(&fixture.run());
    assert_eq!(tree(&fixture.root), before_repeat);
    fixture.teardown_after_assertions();
}
#[test]
fn authorized_fresh_execution_then_exact_repeat_reuses_without_a_second_effect() {
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

    assert_reused_without_effect(&fixture.run());
    assert_eq!(fs::read_dir(scope).unwrap().count(), 0);
    fixture.teardown_after_assertions();
}

#[test]
fn reservation_interruption_reconciles_once_through_the_public_continuation_route() {
    let mut fixture = Fixture::new(
        "reservation-interruption-continuation",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let before = tree(&fixture.root);
    let mut interrupted = fixture.base_command();
    interrupted.args([
        "--json",
        "check",
        "routine",
        "--interrupt-after",
        "reservation",
    ]);
    let interrupted = interrupted.output().unwrap();
    assert_eq!(interrupted.status.code(), Some(1), "{interrupted:?}");
    assert!(interrupted.stderr.is_empty(), "{interrupted:?}");
    let interrupted_value = Fixture::value(&interrupted);
    assert_eq!(
        interrupted_value["schema_version"],
        "RoutinePublicProductionOutcome-v2"
    );
    assert_eq!(interrupted_value["status"], "interrupted-reservation");
    assert_eq!(interrupted_value["effect"], "none");
    assert_eq!(interrupted_value["recovery_required"], true);
    let continuation = interrupted_value["continuation"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(continuation.starts_with("routine-cont-"));
    let reserved: Value = serde_json::from_slice(&fs::read(fixture.checkpoint_path()).unwrap())
        .expect("reservation checkpoint is not JSON");
    assert_eq!(reserved["state"], "reserved");
    assert_eq!(reserved["operation"], "terminal");
    assert!(reserved["terminal_outcome"].is_null());
    assert_eq!(reserved["continuation"], continuation);
    assert!(reserved["attempt_grant"].as_str().is_some());
    assert!(reserved["authenticated_ledger_head"].as_str().is_some());
    assert_eq!(tree(&fixture.root), before);

    let before_foreign = tree(&fixture.root);
    let mut foreign = fixture.base_command();
    foreign.args([
        "--json",
        "check",
        "routine",
        "--continuation",
        "routine-cont-foreign",
    ]);
    let foreign = foreign.output().unwrap();
    assert_eq!(foreign.status.code(), Some(3), "{foreign:?}");
    assert!(foreign.stdout.is_empty(), "{foreign:?}");
    assert_eq!(tree(&fixture.root), before_foreign);

    let mut recovered = fixture.base_command();
    recovered.args([
        "--json",
        "check",
        "routine",
        "--continuation",
        &continuation,
    ]);
    let recovered = recovered.output().unwrap();
    assert_eq!(recovered.status.code(), Some(0), "{recovered:?}");
    assert!(recovered.stderr.is_empty(), "{recovered:?}");
    let recovered_value = Fixture::value(&recovered);
    assert_eq!(recovered_value["status"], "executed");
    assert_eq!(recovered_value["nodes"][0]["disposition"], "executed");

    let before_replay = tree(&fixture.root);
    let mut replay = fixture.base_command();
    replay.args([
        "--json",
        "check",
        "routine",
        "--continuation",
        &continuation,
    ]);
    let replay = replay.output().unwrap();
    assert_eq!(replay.status.code(), Some(3), "{replay:?}");
    assert!(replay.stdout.is_empty(), "{replay:?}");
    assert_eq!(tree(&fixture.root), before_replay);
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
    let terminal_event = read_terminal_event(&fixture);
    assert_eq!(terminal_event["event"]["outcome"], "fail");
    assert_eq!(
        terminal_event["event"]["public_attributes"]["routine_transition"],
        "failed"
    );
    assert_eq!(
        terminal_event["event"]["public_attributes"]["routine_terminal_outcome"],
        "failed"
    );
    let checkpoint: Value = serde_json::from_slice(&fs::read(fixture.checkpoint_path()).unwrap())
        .expect("terminal checkpoint is not JSON");
    assert_eq!(checkpoint["state"], "terminal-event-joined");
    assert_eq!(checkpoint["terminal_outcome"], "failed");
    assert_terminal_refusal(&fixture.run());
    fixture.teardown_after_assertions();
}

#[test]
fn missing_validation_artifacts_ignore_fails_closed_before_the_first_effect() {
    let mut fixture = Fixture::new(
        "missing-observability-ignore",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    fs::write(fixture.root.join(".gitignore"), b"target/\n").unwrap();
    let before_status = fixture.status();
    let refused = fixture.run();
    assert_eq!(refused.status.code(), Some(4), "{refused:?}");
    assert!(refused.stdout.is_empty(), "{refused:?}");
    let value: Value = serde_json::from_slice(&refused.stderr).unwrap();
    assert_eq!(
        value["diagnostic_id"],
        "successor_runtime_downstream_tool_unavailable"
    );
    assert_eq!(
        value["cause"],
        "routine-runtime-observability-store-not-ignored"
    );
    assert_eq!(fixture.status(), before_status);
    assert!(!fixture.root.join("target/routine").exists());
    assert!(!fixture.checkpoint_path().exists());
    assert!(
        !fixture
            .root
            .join("validation_artifacts/observability/spool")
            .exists()
    );
    fixture.teardown_after_assertions();
}

#[test]
fn tracked_runtime_event_store_fails_closed_before_the_first_effect() {
    let mut fixture = Fixture::new(
        "tracked-observability-store",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let store = fixture
        .root
        .join("validation_artifacts/observability/spool/successor-events-forged.jsonl");
    fs::create_dir_all(store.parent().unwrap()).unwrap();
    fs::write(&store, b"tracked-runtime-store\n").unwrap();
    git(
        &fixture.root,
        &[
            "add",
            "-f",
            "validation_artifacts/observability/spool/successor-events-forged.jsonl",
        ],
    );
    git(
        &fixture.root,
        &["commit", "--quiet", "-m", "tracked runtime store"],
    );

    let before_status = fixture.status();
    let refused = fixture.run();
    assert_eq!(refused.status.code(), Some(4), "{refused:?}");
    assert!(refused.stdout.is_empty(), "{refused:?}");
    let value: Value = serde_json::from_slice(&refused.stderr).unwrap();
    assert_eq!(
        value["cause"],
        "routine-runtime-observability-store-not-ignored"
    );
    assert_eq!(fixture.status(), before_status);
    assert_eq!(fs::read(store).unwrap(), b"tracked-runtime-store\n");
    assert!(!fixture.checkpoint_path().exists());
    assert!(!fixture.root.join("target/routine").exists());
    fixture.teardown_after_assertions();
}

fn read_terminal_event(fixture: &Fixture) -> Value {
    let path = terminal_event_path(&fixture.root);
    let text = fs::read_to_string(path).expect("terminal event was not appended");
    serde_json::from_str(text.trim()).expect("terminal event is not JSON")
}

fn terminal_event_path(root: &std::path::Path) -> std::path::PathBuf {
    let spool = root.join("validation_artifacts/observability/spool");
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
    assert_eq!(paths.len(), 1, "fixture has one bound event journal");
    paths.into_iter().next().unwrap()
}

#[test]
fn independent_fresh_authority_roots_execute_once_and_reuse_their_own_bindings() {
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
    assert_reused_without_effect(&first.run());
    assert_reused_without_effect(&second.run());
    second.teardown_after_assertions();
    first.teardown_after_assertions();
}

#[test]
fn shared_host_continuations_keep_foreign_legacy_and_repository_records_independent() {
    let mut first = Fixture::new(
        "shared-home-first",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let mut second = Fixture::new(
        "shared-home-second",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let mut interrupted = first.base_command();
    interrupted.args([
        "--json",
        "check",
        "routine",
        "--interrupt-after",
        "reservation",
    ]);
    let interrupted = interrupted.output().unwrap();
    assert_eq!(interrupted.status.code(), Some(1), "{interrupted:?}");
    let first_record = first.checkpoint_path();
    let legacy = first.state_root().join("adapter/routine-continuation.json");
    fs::copy(&first_record, &legacy).unwrap();

    let mut second_run = routine_command(&second.root, &first.home, second.binary_path());
    second_run.args(["--json", "check", "routine"]);
    let output = second_run.output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(Fixture::value(&output)["status"], "executed");
    assert_eq!(fs::read_dir(first.continuations_root()).unwrap().count(), 2);
    assert_eq!(fs::read(&legacy).unwrap(), fs::read(&first_record).unwrap());

    second.teardown_after_assertions();
    first.teardown_after_assertions();
}

#[test]
fn one_repository_can_reopen_a_distinct_binding_without_replacing_the_prior_record() {
    let mut fixture = Fixture::new(
        "same-repository-distinct-binding",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let first = fixture.run_args(&[
        "--json",
        "check",
        "routine",
        "--interrupt-after",
        "reservation",
    ]);
    assert_eq!(first.status.code(), Some(1), "{first:?}");
    let first_record = fixture.checkpoint_path();
    fs::write(
        fixture.root.join("src/lib.rs"),
        b"pub fn value() -> u8 { 3 }\n",
    )
    .unwrap();
    let second = fixture.run_args(&[
        "--json",
        "check",
        "routine",
        "--interrupt-after",
        "reservation",
    ]);
    assert_eq!(second.status.code(), Some(1), "{second:?}");
    let records = fs::read_dir(fixture.continuations_root())
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    assert_eq!(records.len(), 2);
    assert!(records.iter().any(|path| path == &first_record));
    fixture.teardown_after_assertions();
}

#[test]
fn completed_binding_reuses_after_a_distinct_binding_recovers_without_relaxing_stale_reservations()
{
    let mut fixture = Fixture::new(
        "completed-binding-reuses-after-distinct-recovery",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let original = fixture.run();
    assert_eq!(original.status.code(), Some(0), "{original:?}");
    assert_eq!(Fixture::value(&original)["status"], "executed");

    let control = fixture.root.join("src/recovery-control.rs");
    fs::write(&control, b"pub const RECOVERY_CONTROL: u8 = 1;\n").unwrap();
    let interrupted = fixture.run_args(&[
        "--json",
        "check",
        "routine",
        "--interrupt-after",
        "reservation",
    ]);
    assert_eq!(interrupted.status.code(), Some(1), "{interrupted:?}");
    let continuation = Fixture::value(&interrupted)["continuation"]
        .as_str()
        .unwrap()
        .to_owned();
    let recovered = fixture.run_args(&[
        "--json",
        "check",
        "routine",
        "--continuation",
        &continuation,
    ]);
    assert_eq!(recovered.status.code(), Some(0), "{recovered:?}");
    assert_eq!(Fixture::value(&recovered)["status"], "executed");

    fs::remove_file(&control).unwrap();
    let before_reuse = tree(&fixture.root);
    assert_reused_without_effect(&fixture.run());
    assert_eq!(tree(&fixture.root), before_reuse);

    fixture.teardown_after_assertions();
}

#[test]
fn stale_reserved_binding_refuses_after_a_distinct_binding_advances_the_ledger() {
    let mut fixture = Fixture::new(
        "stale-reserved-binding-refusal",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let interrupted = fixture.run_args(&[
        "--json",
        "check",
        "routine",
        "--interrupt-after",
        "reservation",
    ]);
    assert_eq!(interrupted.status.code(), Some(1), "{interrupted:?}");
    let continuation = Fixture::value(&interrupted)["continuation"]
        .as_str()
        .unwrap()
        .to_owned();

    let control = fixture.root.join("src/ledger-advance.rs");
    fs::write(&control, b"pub const LEDGER_ADVANCE: u8 = 1;\n").unwrap();
    let advanced = fixture.run();
    assert_eq!(advanced.status.code(), Some(0), "{advanced:?}");
    assert_eq!(Fixture::value(&advanced)["status"], "executed");

    fs::remove_file(&control).unwrap();
    let stale = fixture.run_args(&[
        "--json",
        "check",
        "routine",
        "--continuation",
        &continuation,
    ]);
    assert_eq!(stale.status.code(), Some(3), "{stale:?}");
    assert!(stale.stdout.is_empty(), "{stale:?}");
    fixture.teardown_after_assertions();
}

#[test]
fn exact_legacy_checkpoint_remains_recoverable_at_the_compatibility_boundary() {
    let mut fixture = Fixture::new(
        "legacy-continuation-compatibility",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let mut interrupted = fixture.base_command();
    interrupted.args([
        "--json",
        "check",
        "routine",
        "--interrupt-after",
        "reservation",
    ]);
    let interrupted = interrupted.output().unwrap();
    let continuation = Fixture::value(&interrupted)["continuation"]
        .as_str()
        .unwrap()
        .to_owned();
    let canonical = fixture.checkpoint_path();
    let legacy = fixture
        .state_root()
        .join("adapter/routine-continuation.json");
    fs::copy(&canonical, &legacy).unwrap();
    fs::remove_file(canonical).unwrap();

    let mut recovered = fixture.base_command();
    recovered.args([
        "--json",
        "check",
        "routine",
        "--continuation",
        &continuation,
    ]);
    let recovered = recovered.output().unwrap();
    assert_eq!(recovered.status.code(), Some(0), "{recovered:?}");
    assert_eq!(Fixture::value(&recovered)["status"], "executed");
    assert!(legacy.is_file());
    assert_eq!(
        fs::read_dir(fixture.continuations_root()).unwrap().count(),
        0
    );
    fixture.teardown_after_assertions();
}

fn assert_terminal_refusal(output: &std::process::Output) {
    assert_eq!(output.status.code(), Some(3), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    assert!(
        serde_json::from_slice::<Value>(&output.stderr).is_ok(),
        "{output:?}"
    );
}

fn assert_reused_without_effect(output: &std::process::Output) {
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let value = Fixture::value(output);
    assert_eq!(value["status"], "reused");
    assert_eq!(value["effect"], "none");
    assert_eq!(value["nodes"][0]["disposition"], "reused");
}

fn assert_status(fixture: &Fixture, expected: &str) {
    let output = fixture.run();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    assert_eq!(Fixture::value(&output)["status"], expected);
}
