use super::fresh_process_child::{CHILD_ENV, CHILD_TEST, ChildInput, ChildOutput};
use super::support::*;
use crate::orchestration::*;
use std::collections::BTreeSet;
use std::fs;
use std::process::Command;

fn input(
    mode: &str,
    journal: &TestRoot,
    output: std::path::PathBuf,
    head: JournalHead,
    tick: u64,
) -> ChildInput {
    ChildInput {
        mode: mode.to_owned(),
        journal_root: journal.path().to_path_buf(),
        output,
        expected_head: head,
        expected_event_id: None,
        recovered_binding: None,
        resolution: None,
        tick,
        live_workers: BTreeSet::from(["worker-a".to_owned()]),
        action: None,
        permit: None,
    }
}

fn run_child(control: &TestRoot, label: &str, input: &ChildInput) -> ChildOutput {
    let input_path = control.path().join(format!("{label}-input.json"));
    fs::write(&input_path, serde_json::to_vec(input).unwrap()).unwrap();
    let status = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", CHILD_TEST, "--nocapture"])
        .env(CHILD_ENV, &input_path)
        .status()
        .unwrap();
    assert!(status.success());
    serde_json::from_slice(&fs::read(&input.output).unwrap()).unwrap()
}

#[test]
fn interrupted_root_inspect_authorize_execute_and_reopen_cross_processes() {
    let (journal, head, commitment) = interrupted_root("fresh-resume");
    let control = TestRoot::new("fresh-resume-control");
    let before_read = recursive_fingerprint(journal.path());

    let inspect_path = control.path().join("resume-inspect.json");
    let inspected = run_child(
        &control,
        "resume-inspect",
        &input("inspect-current", &journal, inspect_path, head.clone(), 5),
    );
    assert_eq!(inspected.status, "inspected-current");
    assert_eq!(inspected.read_unchanged, Some(true));
    assert_eq!(recursive_fingerprint(journal.path()), before_read);

    let mut execute = input(
        "resume",
        &journal,
        control.path().join("resume-execute.json"),
        head,
        5,
    );
    execute.action = inspected.action;
    execute.permit = inspected.permit;
    let resumed = run_child(&control, "resume-execute", &execute);
    assert_eq!(resumed.status, "resumed");
    let snapshot = resumed.snapshot.unwrap();
    assert!(!snapshot.recovery.interrupted_root);
    assert_eq!(
        snapshot.commitments["lease-001"].result_commitment_id,
        commitment
    );

    let reopened = run_child(
        &control,
        "resume-reopen",
        &input(
            "reopen",
            &journal,
            control.path().join("resume-reopen.json"),
            resumed.current_head.unwrap(),
            5,
        ),
    );
    assert_eq!(reopened.status, "reopened");
    assert_eq!(reopened.read_unchanged, Some(true));
    assert!(!reopened.snapshot.unwrap().recovery.interrupted_root);
}

#[test]
fn ambiguity_inspect_authorize_reconcile_and_reopen_cross_processes() {
    let (journal, head) = ambiguous_effect("fresh-reconcile");
    let control = TestRoot::new("fresh-reconcile-control");
    let before_read = recursive_fingerprint(journal.path());
    let resolution = not_applied_resolution();
    let mut inspect = input(
        "inspect-current",
        &journal,
        control.path().join("reconcile-inspect.json"),
        head.clone(),
        4,
    );
    inspect.resolution = Some(resolution.clone());
    let inspected = run_child(&control, "reconcile-inspect", &inspect);
    assert_eq!(inspected.status, "inspected-current");
    assert_eq!(inspected.read_unchanged, Some(true));
    assert_eq!(recursive_fingerprint(journal.path()), before_read);

    let mut execute = input(
        "reconcile",
        &journal,
        control.path().join("reconcile-execute.json"),
        head,
        4,
    );
    execute.action = inspected.action;
    execute.permit = inspected.permit;
    execute.resolution = Some(resolution);
    let reconciled = run_child(&control, "reconcile-execute", &execute);
    assert_eq!(reconciled.status, "reconciled");

    let reopened = run_child(
        &control,
        "reconcile-reopen",
        &input(
            "reopen",
            &journal,
            control.path().join("reconcile-reopen.json"),
            reconciled.current_head.unwrap(),
            4,
        ),
    );
    assert_eq!(reopened.status, "reopened");
    assert_eq!(reopened.read_unchanged, Some(true));
    let snapshot = reopened.snapshot.unwrap();
    assert!(snapshot.recovery.ambiguous_operations.is_empty());
    assert!(snapshot.settled_operations.contains("operation-001"));
}

#[test]
fn interrupted_append_inspect_authorize_recover_and_reopen_cross_processes() {
    let (journal, prior, event, recovery) = interrupted_heartbeat("fresh-recover");
    let control = TestRoot::new("fresh-recover-control");
    let before_read = recursive_fingerprint(journal.path());
    let mut inspect = input(
        "inspect-interrupted",
        &journal,
        control.path().join("recover-inspect.json"),
        prior.clone(),
        recovery.tick,
    );
    inspect.expected_event_id = Some(recovery.expected_event_id.clone());
    inspect.recovered_binding = Some(recovery.recovered_binding.clone());
    let inspected = run_child(&control, "recover-inspect", &inspect);
    assert_eq!(inspected.status, "inspected-interrupted");
    assert_eq!(inspected.read_unchanged, Some(true));
    assert_eq!(recursive_fingerprint(journal.path()), before_read);

    let mut execute = input(
        "recover",
        &journal,
        control.path().join("recover-execute.json"),
        prior.clone(),
        recovery.tick,
    );
    execute.expected_event_id = Some(recovery.expected_event_id);
    execute.recovered_binding = Some(recovery.recovered_binding);
    execute.action = inspected.action;
    execute.permit = inspected.permit;
    let recovered = run_child(&control, "recover-execute", &execute);
    assert_eq!(recovered.status, "recovered");
    let current = recovered.current_head.unwrap();
    assert_eq!(current.last_event_id, event.event_id);
    assert_eq!(current.event_count, prior.event_count + 1);

    let reopened = run_child(
        &control,
        "recover-reopen",
        &input(
            "reopen",
            &journal,
            control.path().join("recover-reopen.json"),
            current,
            recovery.tick,
        ),
    );
    assert_eq!(reopened.status, "reopened");
    assert_eq!(reopened.read_unchanged, Some(true));
    assert_eq!(reopened.snapshot.unwrap().binding, binding());
}
