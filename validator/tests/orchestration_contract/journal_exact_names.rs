use crate::durable_support::*;
use crate::orchestration::*;
use crate::support::*;
use std::fs;

const NAMES: [(&str, &str); 3] = [
    ("events.jsonl", "EVENTS.JSONL"),
    ("head.json", "HEAD.JSON"),
    ("journal.lock", "JOURNAL.LOCK"),
];

fn require_case_insensitive_fixture() {
    let root = JournalRoot::new("case-insensitive-proof");
    fs::create_dir(root.path()).unwrap();
    let lower = root.path().join("case-probe");
    let upper = root.path().join("CASE-PROBE");
    fs::write(&lower, b"probe").unwrap();
    let error = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&upper)
        .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
}

fn rename_case(root: &JournalRoot, from: &str, to: &str) {
    fs::rename(root.path().join(from), root.path().join(to)).unwrap();
    let names: Vec<_> = fs::read_dir(root.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    assert!(names.iter().any(|name| name == to));
    assert!(!names.iter().any(|name| name == from));
}

fn journal_bytes(root: &JournalRoot) -> (Vec<u8>, Vec<u8>) {
    (
        fs::read(root.path().join("events.jsonl")).unwrap(),
        fs::read(root.path().join("head.json")).unwrap(),
    )
}

fn interrupted(label: &str) -> (JournalRoot, JournalHead, OrchestrationEvent) {
    let (mut engine, _, root_dir) = durable_engine(label);
    engine.grant_lease(1, lease()).unwrap();
    engine.start(2, "lease-001").unwrap();
    let prior = engine.journal_head().unwrap().clone();
    let log = engine.event_log();
    let event = log
        .next(
            &binding(),
            worker("worker-a"),
            3,
            EventKind::Heartbeat {
                lease_id: "lease-001".to_owned(),
            },
        )
        .unwrap();
    drop(engine);
    let frame = serde_json::json!({
        "schema_version": "OrchestrationJournalFrame-v1",
        "event": event,
    });
    let mut bytes = fs::read(root_dir.path().join("events.jsonl")).unwrap();
    bytes.extend(serde_json::to_vec(&frame).unwrap());
    bytes.push(b'\n');
    fs::write(root_dir.path().join("events.jsonl"), bytes).unwrap();
    (root_dir, prior, event)
}

#[test]
fn create_rejects_case_aliases_for_every_authority_name() {
    require_case_insensitive_fixture();
    for (exact, alias) in NAMES {
        let root_dir = JournalRoot::new(exact);
        fs::create_dir(root_dir.path()).unwrap();
        fs::write(root_dir.path().join(alias), b"alias").unwrap();
        let (sink, _) = CountingSink::new();
        assert!(
            Orchestrator::new_durable(
                graph_one(),
                policy(),
                binding(),
                root(),
                bootstrap(),
                root_dir.path(),
                sink,
            )
            .is_err()
        );
        let names: Vec<_> = fs::read_dir(root_dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        assert!(names.iter().any(|name| name == alias));
        assert!(!names.iter().any(|name| name == exact));
    }
}

#[test]
fn strict_open_and_anchored_inspect_reject_each_case_alias() {
    require_case_insensitive_fixture();
    for (exact, alias) in NAMES {
        let (_engine, _, root_dir) = durable_engine(alias);
        let journal = FileJournal::open(root_dir.path()).unwrap();
        rename_case(&root_dir, exact, alias);
        assert_eq!(
            FileJournal::open(root_dir.path()).unwrap_err(),
            OrchestrationError::JournalCorrupt
        );
        assert_eq!(
            journal.inspect().unwrap_err(),
            OrchestrationError::JournalCorrupt
        );
        rename_case(&root_dir, alias, exact);
    }
}

#[test]
fn append_rejects_each_case_alias_without_changing_journal_bytes() {
    require_case_insensitive_fixture();
    for (exact, alias) in NAMES {
        let (mut engine, _, root_dir) = durable_engine(alias);
        engine.grant_lease(1, lease()).unwrap();
        engine.start(2, "lease-001").unwrap();
        let before = journal_bytes(&root_dir);
        rename_case(&root_dir, exact, alias);
        assert!(engine.heartbeat(3, "lease-001").is_err());
        assert_eq!(journal_bytes(&root_dir), before);
        rename_case(&root_dir, alias, exact);
    }
}

#[test]
fn recovery_opener_rejects_each_case_alias_without_repairing() {
    require_case_insensitive_fixture();
    for (exact, alias) in NAMES {
        let (root_dir, prior, event) = interrupted(alias);
        let before = journal_bytes(&root_dir);
        rename_case(&root_dir, exact, alias);
        assert!(
            FileJournal::recover_interrupted_append(
                root_dir.path(),
                &prior,
                &event.event_id,
                &binding(),
            )
            .is_err()
        );
        assert_eq!(journal_bytes(&root_dir), before);
        rename_case(&root_dir, alias, exact);
    }
}

#[test]
fn alias_mutation_between_enumeration_and_open_fails_closed() {
    require_case_insensitive_fixture();
    let (_engine, _, root_dir) = durable_engine("name-open-race");
    let journal = FileJournal::open(root_dir.path()).unwrap();
    let before = journal_bytes(&root_dir);
    assert_eq!(
        journal
            .inspect_with_log_name_hook(|| {
                rename_case(&root_dir, "events.jsonl", "EVENTS.JSONL");
            })
            .unwrap_err(),
        OrchestrationError::JournalCorrupt
    );
    assert_eq!(journal_bytes(&root_dir), before);
    rename_case(&root_dir, "EVENTS.JSONL", "events.jsonl");
    journal.inspect().unwrap();
}
