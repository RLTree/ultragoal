use crate::durable_journal_fixture::*;
use crate::orchestration::*;
use crate::orchestration_fixture::*;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;
use std::sync::{Arc, Barrier};

const CHILD_MODE: &str = "HUL_ORCHESTRATION_EFFECT_CRASH_CHILD";
const CHILD_PATH: &str = "HUL_ORCHESTRATION_JOURNAL_PATH";
const CHILD_HEAD: &str = "HUL_ORCHESTRATION_EXPECTED_HEAD";

struct ExitSink;

impl EffectSink for ExitSink {
    fn apply(&mut self, _request: &EffectRequest) -> Result<EffectReceipt, OrchestrationError> {
        std::process::exit(86)
    }
}

fn effect_request() -> EffectRequest {
    EffectRequest {
        lease_id: "lease-001".to_owned(),
        binding: binding(),
        effect: effect(EffectClass::WorkspaceWrite, "leased-source/node_a"),
        operation_id: "operation-crash-001".to_owned(),
        payload_digest: digest('7'),
    }
}

#[test]
fn effect_intent_is_durable_across_process_death_inside_sink() {
    if std::env::var_os(CHILD_MODE).is_some() {
        let path = std::env::var(CHILD_PATH).unwrap();
        let expected: JournalHead =
            serde_json::from_str(&std::env::var(CHILD_HEAD).unwrap()).unwrap();
        let mut engine =
            Orchestrator::restart_durable(graph_one(), policy(), expected, root(), path, ExitSink)
                .unwrap();
        let _ = engine.apply_effect(3, effect_request());
        panic!("exit sink returned");
    }

    let (mut engine, _, journal_root) = durable_engine("effect-process-crash");
    engine.grant_lease(1, lease()).unwrap();
    engine.start(2, "lease-001").unwrap();
    let prior_head = engine.journal_head().unwrap().clone();
    let prior_log = engine.event_log();
    let request = effect_request();
    let expected_event = OrchestrationEvent::create(
        prior_log.events().len() as u64,
        prior_log
            .events()
            .last()
            .map(|event| event.event_id.clone()),
        binding(),
        worker("worker-a"),
        3,
        EventKind::EffectIntent {
            lease_id: request.lease_id.clone(),
            request,
        },
    )
    .unwrap();
    drop(engine);

    let status = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("journal_recovery::effect_intent_is_durable_across_process_death_inside_sink")
        .arg("--nocapture")
        .env(CHILD_MODE, "1")
        .env(CHILD_PATH, journal_root.path())
        .env(CHILD_HEAD, serde_json::to_string(&prior_head).unwrap())
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(86));

    let journal = FileJournal::open(journal_root.path()).unwrap();
    let recovered_head = journal
        .verify_single_advance(&prior_head, &expected_event.event_id, &binding())
        .unwrap();
    let (sink, calls) = CountingSink::new();
    let restarted = Orchestrator::restart_durable(
        graph_one(),
        policy(),
        recovered_head,
        root(),
        journal_root.path(),
        sink,
    )
    .unwrap();
    assert_eq!(calls.get(), 0);
    assert_eq!(
        restarted
            .recovery_report(&binding(), 3, &BTreeSet::new())
            .ambiguous_operations,
        ["operation-crash-001"]
    );
}

#[test]
fn trusted_head_rejects_a_forged_valid_prefix() {
    let (mut engine, _, journal_root) = durable_engine("forged-prefix");
    engine.grant_lease(1, lease()).unwrap();
    engine.start(2, "lease-001").unwrap();
    let trusted = engine.journal_head().unwrap().clone();
    drop(engine);

    let journal = FileJournal::open(journal_root.path()).unwrap();
    let snapshot = journal.inspect().unwrap();
    let mut lines: Vec<_> = std::fs::read(journal_root.path().join("events.jsonl"))
        .unwrap()
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(<[u8]>::to_vec)
        .collect();
    lines.pop();
    let mut prefix_bytes = lines.join(&b'\n');
    prefix_bytes.push(b'\n');
    let prefix_last = &snapshot.log.events()[snapshot.log.events().len() - 2];
    let forged = JournalHead {
        schema_version: "OrchestrationJournalHead-v1".to_owned(),
        binding: binding(),
        event_count: trusted.event_count - 1,
        last_event_id: prefix_last.event_id.clone(),
        log_sha256: format!("sha256:{:x}", Sha256::digest(&prefix_bytes)),
    };
    std::fs::write(journal_root.path().join("events.jsonl"), prefix_bytes).unwrap();
    let mut head_bytes = serde_json::to_vec(&forged).unwrap();
    head_bytes.push(b'\n');
    std::fs::write(journal_root.path().join("head.json"), head_bytes).unwrap();
    FileJournal::open(journal_root.path()).unwrap();
    let (sink, _) = CountingSink::new();
    assert_eq!(
        Orchestrator::restart_durable(
            graph_one(),
            policy(),
            trusted,
            root(),
            journal_root.path(),
            sink,
        )
        .err()
        .unwrap(),
        OrchestrationError::JournalConflict
    );
}

#[test]
fn concurrent_expected_head_append_has_exactly_one_winner() {
    let (mut engine, _, journal_root) = durable_engine("concurrent-append");
    engine.grant_lease(1, lease()).unwrap();
    engine.start(2, "lease-001").unwrap();
    let base = engine.event_log();
    let expected = engine.journal_head().unwrap().clone();
    drop(engine);

    let make_log = |tick| {
        let next = base
            .next(
                &binding(),
                worker("worker-a"),
                tick,
                EventKind::Heartbeat {
                    lease_id: "lease-001".to_owned(),
                },
            )
            .unwrap();
        let mut log = base.clone();
        log.0.push(next);
        log
    };
    let barrier = Arc::new(Barrier::new(2));
    let mut handles = Vec::new();
    for log in [make_log(3), make_log(4)] {
        let journal = FileJournal::open(journal_root.path()).unwrap();
        let expected = expected.clone();
        let barrier = Arc::clone(&barrier);
        handles.push(std::thread::spawn(move || {
            barrier.wait();
            journal.append(&expected, &binding(), &log)
        }));
    }
    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter_map(|result| result.as_ref().err())
            .copied()
            .collect::<Vec<_>>(),
        [OrchestrationError::JournalConflict]
    );
}

#[test]
fn journal_open_and_inspect_are_byte_for_byte_zero_write() {
    let (_engine, _, journal_root) = durable_engine("zero-write-inspect");
    let snapshot = || -> BTreeMap<String, (Vec<u8>, std::time::SystemTime)> {
        std::fs::read_dir(journal_root.path())
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                let name = entry.file_name().into_string().unwrap();
                let metadata = entry.metadata().unwrap();
                (
                    name,
                    (
                        std::fs::read(entry.path()).unwrap(),
                        metadata.modified().unwrap(),
                    ),
                )
            })
            .collect()
    };
    let before = snapshot();
    let journal = FileJournal::open(journal_root.path()).unwrap();
    let _ = journal.inspect().unwrap();
    assert_eq!(snapshot(), before);
}
