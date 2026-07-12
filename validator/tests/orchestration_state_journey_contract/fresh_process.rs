use super::fresh_process_child::{CHILD_ENV, CHILD_TEST, ChildInput, ChildOutput};
use super::support::*;
use crate::orchestration::product::*;
use crate::orchestration::*;
use serde::Serialize;
use std::collections::BTreeSet;
use std::fs;
use std::process::{Child, Command};

fn write_input(control: &TestRoot, name: &str, input: &ChildInput) -> std::path::PathBuf {
    let path = control.path().join(format!("{name}-input.json"));
    fs::write(&path, serde_json::to_vec(input).unwrap()).unwrap();
    path
}

fn spawn_child(input_path: &std::path::Path) -> Child {
    Command::new(std::env::current_exe().unwrap())
        .args(["--exact", CHILD_TEST, "--nocapture"])
        .env(CHILD_ENV, input_path)
        .spawn()
        .unwrap()
}

fn run_child(input_path: &std::path::Path, output_path: &std::path::Path) -> ChildOutput {
    let status = spawn_child(input_path).wait().unwrap();
    assert!(status.success());
    serde_json::from_slice(&fs::read(output_path).unwrap()).unwrap()
}

fn input(
    mode: &str,
    journal_root: &TestRoot,
    output: std::path::PathBuf,
    expected_head: JournalHead,
    tick: u64,
) -> ChildInput {
    ChildInput {
        mode: mode.to_owned(),
        journal_root: journal_root.path().to_path_buf(),
        output,
        expected_head,
        expected_event_id: None,
        recovered_binding: None,
        tick,
        live_workers: BTreeSet::from(["worker-a".to_owned()]),
    }
}

#[test]
fn interrupted_root_restarts_in_a_fresh_process_with_all_identity_bindings() {
    let (journal, mut engine) = durable_engine("fresh-resume", false);
    let artifacts = TestRoot::new("fresh-resume-artifacts");
    let result = worker_result(&artifacts);
    let lease = lease(40);
    engine.grant_lease(1, lease.clone()).unwrap();
    engine.start(2, &lease.lease_id).unwrap();
    engine
        .submit(
            3,
            &lease.lease_id,
            &result,
            &ArtifactWorkspace::new(artifacts.path()).unwrap(),
        )
        .unwrap();
    let commitment = engine
        .events()
        .iter()
        .find_map(|event| match &event.event {
            EventKind::WorkerSubmitted {
                result_commitment_id,
                ..
            } => Some(result_commitment_id.clone()),
            _ => None,
        })
        .unwrap();
    engine.interrupt_root(4).unwrap();
    let head = engine.journal_head().unwrap().clone();
    drop(engine);
    let workspace_identity = ProductWorkspace::open(journal.path())
        .unwrap()
        .identity()
        .to_owned();
    let control = TestRoot::new("fresh-resume-control");
    let output_path = control.path().join("resume-output.json");
    let input = input("resume", &journal, output_path.clone(), head, 5);
    let input_path = write_input(&control, "resume", &input);
    let output = run_child(&input_path, &output_path);

    assert_eq!(output.status, "resumed");
    assert_eq!(
        output.workspace_identity.as_deref(),
        Some(workspace_identity.as_str())
    );
    let action = output.action.unwrap();
    assert_eq!(action.authority_binding, binding());
    assert_eq!(action.target.lease_id.as_deref(), Some("lease-001"));
    assert_eq!(
        action.target.result_commitment_id.as_ref(),
        Some(&commitment)
    );
    let snapshot = output.snapshot.unwrap();
    assert_eq!(snapshot.binding, binding());
    assert_eq!(
        snapshot.commitments["lease-001"].result_commitment_id,
        commitment
    );
    assert!(!snapshot.recovery.interrupted_root);
}

#[test]
fn ambiguous_effect_is_inspected_then_reconciled_across_fresh_processes() {
    let (journal, mut engine) = durable_engine("fresh-reconcile", true);
    engine.grant_lease(1, lease(40)).unwrap();
    engine.start(2, "lease-001").unwrap();
    let request = EffectRequest {
        lease_id: "lease-001".to_owned(),
        binding: binding(),
        effect: effect(),
        operation_id: "operation-001".to_owned(),
        payload_digest: digest('6'),
    };
    assert_eq!(
        engine.apply_effect(3, request).unwrap_err(),
        OrchestrationError::EffectAmbiguous
    );
    let head = engine.journal_head().unwrap().clone();
    drop(engine);
    let before = recursive_fingerprint(journal.path());
    let control = TestRoot::new("fresh-reconcile-control");

    let inspect_output = control.path().join("inspect-output.json");
    let inspect_input = input("inspect", &journal, inspect_output.clone(), head.clone(), 4);
    let inspect_input_path = write_input(&control, "inspect", &inspect_input);
    let inspected = run_child(&inspect_input_path, &inspect_output);
    assert_eq!(inspected.status, "inspected");
    let action = inspected.action.unwrap();
    assert_eq!(action.operation, RootOperation::Reconcile);
    assert_eq!(action.target.lease_id.as_deref(), Some("lease-001"));
    assert_eq!(action.target.operation_id.as_deref(), Some("operation-001"));
    assert_eq!(recursive_fingerprint(journal.path()), before);

    let reconcile_output = control.path().join("reconcile-output.json");
    let reconcile_input = input("reconcile", &journal, reconcile_output.clone(), head, 4);
    let reconcile_input_path = write_input(&control, "reconcile", &reconcile_input);
    let reconciled = run_child(&reconcile_input_path, &reconcile_output);
    assert_eq!(reconciled.status, "reconciled");

    let current = reconciled.current_head.unwrap();
    let clean_output = control.path().join("clean-output.json");
    let clean_input = input("inspect", &journal, clean_output.clone(), current, 4);
    let clean_input_path = write_input(&control, "clean", &clean_input);
    let clean = run_child(&clean_input_path, &clean_output);
    assert_eq!(clean.status, "inspected");
    assert!(
        clean
            .snapshot
            .unwrap()
            .recovery
            .ambiguous_operations
            .is_empty()
    );
    assert!(clean.action.is_none());
}

#[test]
fn concurrent_fresh_process_recovery_has_one_authoritative_outcome() {
    let (journal, mut engine) = durable_engine("fresh-recover", false);
    engine.grant_lease(1, lease(40)).unwrap();
    engine.start(2, "lease-001").unwrap();
    let prior = engine.journal_head().unwrap().clone();
    let event = engine
        .event_log()
        .next(
            &binding(),
            worker_actor(),
            3,
            EventKind::Heartbeat {
                lease_id: "lease-001".to_owned(),
            },
        )
        .unwrap();
    drop(engine);
    let mut bytes = fs::read(journal.path().join("events.jsonl")).unwrap();
    bytes.extend(journal_frame_bytes(&event));
    bytes.push(b'\n');
    fs::write(journal.path().join("events.jsonl"), bytes).unwrap();
    let workspace_identity = ProductWorkspace::open(journal.path())
        .unwrap()
        .identity()
        .to_owned();
    let control = TestRoot::new("fresh-recover-control");

    let mut inputs = Vec::new();
    for index in 0..2 {
        let output = control.path().join(format!("recover-{index}-output.json"));
        let mut request = input("recover", &journal, output.clone(), prior.clone(), 3);
        request.expected_event_id = Some(event.event_id.clone());
        request.recovered_binding = Some(binding());
        let path = write_input(&control, &format!("recover-{index}"), &request);
        inputs.push((path, output));
    }
    let mut children = inputs
        .iter()
        .map(|(path, _)| spawn_child(path))
        .collect::<Vec<_>>();
    for child in &mut children {
        assert!(child.wait().unwrap().success());
    }
    let outputs = inputs
        .iter()
        .map(|(_, output)| {
            serde_json::from_slice::<ChildOutput>(&fs::read(output).unwrap()).unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        outputs
            .iter()
            .filter(|output| output.status == "recovered")
            .count(),
        1
    );
    assert_eq!(
        outputs
            .iter()
            .filter(|output| output.error_code.as_deref() == Some("HUL-ORCH-PROD-005"))
            .count(),
        1
    );
    let recovered = outputs
        .iter()
        .find(|output| output.status == "recovered")
        .unwrap();
    assert_eq!(
        recovered.workspace_identity.as_deref(),
        Some(workspace_identity.as_str())
    );
    assert_eq!(recovered.snapshot.as_ref().unwrap().binding, binding());
    let committed = FileJournal::open(journal.path())
        .unwrap()
        .inspect()
        .unwrap();
    assert_eq!(committed.head, *recovered.current_head.as_ref().unwrap());
    assert_eq!(committed.head.event_count, prior.event_count + 1);
}

fn journal_frame_bytes(event: &OrchestrationEvent) -> Vec<u8> {
    #[derive(Serialize)]
    struct Frame<'a> {
        schema_version: &'static str,
        event: &'a OrchestrationEvent,
    }
    serde_json::to_vec(&Frame {
        schema_version: "OrchestrationJournalFrame-v1",
        event,
    })
    .unwrap()
}
