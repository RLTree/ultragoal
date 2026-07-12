use crate::durable_support::*;
use crate::orchestration::*;
use crate::support::*;
use std::fs;
use std::process::Command;
use std::sync::{Arc, Barrier};

const CHILD_MODE: &str = "HUL_ORCHESTRATION_PUBLICATION_CRASH_CHILD";
const CHILD_PATH: &str = "HUL_ORCHESTRATION_PUBLICATION_CRASH_PATH";
const CHILD_HEAD: &str = "HUL_ORCHESTRATION_PUBLICATION_CRASH_HEAD";

fn heartbeat_after(log: &EventLog) -> OrchestrationEvent {
    log.next(
        &binding(),
        worker("worker-a"),
        3,
        EventKind::Heartbeat {
            lease_id: "lease-001".to_owned(),
        },
    )
    .unwrap()
}

fn initialized(label: &str) -> (JournalRoot, JournalHead, EventLog, OrchestrationEvent) {
    let (mut engine, _, journal) = durable_engine(label);
    engine.grant_lease(1, lease()).unwrap();
    engine.start(2, "lease-001").unwrap();
    let head = engine.journal_head().unwrap().clone();
    let log = engine.event_log();
    let event = heartbeat_after(&log);
    drop(engine);
    (journal, head, log, event)
}

fn publish_interrupted_manually(root: &JournalRoot, log: &EventLog, event: &OrchestrationEvent) {
    #[derive(serde::Serialize)]
    struct Frame<'a> {
        schema_version: &'static str,
        event: &'a OrchestrationEvent,
    }
    let mut bytes = fs::read(root.path().join("events.jsonl")).unwrap();
    bytes.extend(
        serde_json::to_vec(&Frame {
            schema_version: "OrchestrationJournalFrame-v1",
            event,
        })
        .unwrap(),
    );
    bytes.push(b'\n');
    assert_eq!(
        log.events().len() + 1,
        bytes.iter().filter(|b| **b == b'\n').count()
    );
    fs::write(root.path().join("events.jsonl"), bytes).unwrap();
}

fn journal_files(root: &JournalRoot) -> (Vec<u8>, Vec<u8>) {
    (
        fs::read(root.path().join("events.jsonl")).unwrap(),
        fs::read(root.path().join("head.json")).unwrap(),
    )
}

#[test]
fn process_death_between_log_and_head_publication_repairs_exactly_once() {
    if std::env::var_os(CHILD_MODE).is_some() {
        let path = std::env::var(CHILD_PATH).unwrap();
        let expected: JournalHead =
            serde_json::from_str(&std::env::var(CHILD_HEAD).unwrap()).unwrap();
        let journal = FileJournal::open(path).unwrap();
        let snapshot = journal.inspect().unwrap();
        assert_eq!(snapshot.head, expected);
        let event = heartbeat_after(&snapshot.log);
        let mut next = snapshot.log;
        next.0.push(event);
        journal.crash_after_log_publication(&expected, &binding(), &next);
    }

    let (journal, prior, log, event) = initialized("publication-process-death");
    let status = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg(
            "journal_publication_recovery::process_death_between_log_and_head_publication_repairs_exactly_once",
        )
        .arg("--nocapture")
        .env(CHILD_MODE, "1")
        .env(CHILD_PATH, journal.path())
        .env(CHILD_HEAD, serde_json::to_string(&prior).unwrap())
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(87));
    assert_eq!(
        FileJournal::open(journal.path()).unwrap_err(),
        OrchestrationError::JournalCorrupt
    );

    let broken = journal_files(&journal);
    let mut wrong_head = prior.clone();
    wrong_head.last_event_id = digest('8');
    for result in [
        FileJournal::recover_interrupted_append(
            journal.path(),
            &wrong_head,
            &event.event_id,
            &binding(),
        ),
        FileJournal::recover_interrupted_append(journal.path(), &prior, &digest('8'), &binding()),
        FileJournal::recover_interrupted_append(
            journal.path(),
            &prior,
            &event.event_id,
            &Binding::new(&digest('d'), &digest('e')).unwrap(),
        ),
    ] {
        assert!(result.is_err());
        assert_eq!(journal_files(&journal), broken);
    }

    let barrier = Arc::new(Barrier::new(2));
    let mut handles = Vec::new();
    for _ in 0..2 {
        let path = journal.path().to_path_buf();
        let prior = prior.clone();
        let event_id = event.event_id.clone();
        let barrier = Arc::clone(&barrier);
        handles.push(std::thread::spawn(move || {
            barrier.wait();
            FileJournal::recover_interrupted_append(path, &prior, &event_id, &binding())
        }));
    }
    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
    let repaired = FileJournal::open(journal.path())
        .unwrap()
        .inspect()
        .unwrap();
    assert_eq!(repaired.log.events().len(), log.events().len() + 1);
    assert_eq!(repaired.head.last_event_id, event.event_id);
    let (sink, _) = CountingSink::new();
    Orchestrator::restart_durable(
        graph_one(),
        policy(),
        repaired.head,
        root(),
        journal.path(),
        sink,
    )
    .unwrap();
}

#[test]
fn extra_truncated_substituted_or_noncanonical_publications_never_repair() {
    let (root, prior, log, event) = initialized("publication-tamper");
    publish_interrupted_manually(&root, &log, &event);
    let valid = fs::read(root.path().join("events.jsonl")).unwrap();
    let mut cases = Vec::new();
    let mut extra = valid.clone();
    extra.extend_from_slice(valid.split(|byte| *byte == b'\n').next().unwrap());
    extra.push(b'\n');
    cases.push(extra);
    cases.push(valid[..valid.len() - 1].to_vec());
    let mut substituted = valid.clone();
    let index = substituted.iter().position(|byte| *byte == b'3').unwrap();
    substituted[index] = b'4';
    cases.push(substituted);
    let mut noncanonical = valid.clone();
    noncanonical.splice(0..0, [b' ']);
    cases.push(noncanonical);

    let head = fs::read(root.path().join("head.json")).unwrap();
    for bytes in cases {
        fs::write(root.path().join("events.jsonl"), &bytes).unwrap();
        assert!(
            FileJournal::recover_interrupted_append(
                root.path(),
                &prior,
                &event.event_id,
                &binding(),
            )
            .is_err()
        );
        assert_eq!(fs::read(root.path().join("events.jsonl")).unwrap(), bytes);
        assert_eq!(fs::read(root.path().join("head.json")).unwrap(), head);
    }
}

#[test]
fn prepared_recovery_cannot_follow_a_regular_root_substitution() {
    let (root, prior, log, event) = initialized("prepared-root-substitution");
    publish_interrupted_manually(&root, &log, &event);
    let prepared =
        FileJournal::prepare_interrupted_append(root.path(), &prior, &event.event_id, &binding())
            .unwrap();
    let names = ["events.jsonl", "head.json", "journal.lock"];
    let original_before = names.map(|name| fs::read(root.path().join(name)).unwrap());
    let moved = root.path().with_extension("anchored");
    fs::rename(root.path(), &moved).unwrap();
    fs::create_dir(root.path()).unwrap();
    for name in names {
        fs::copy(moved.join(name), root.path().join(name)).unwrap();
    }
    let replacement_before = names.map(|name| fs::read(root.path().join(name)).unwrap());

    assert_eq!(
        prepared.commit().unwrap_err(),
        OrchestrationError::JournalCorrupt
    );
    for (index, name) in names.into_iter().enumerate() {
        assert_eq!(fs::read(moved.join(name)).unwrap(), original_before[index]);
        assert_eq!(
            fs::read(root.path().join(name)).unwrap(),
            replacement_before[index]
        );
    }

    fs::remove_dir_all(root.path()).unwrap();
    fs::rename(moved, root.path()).unwrap();
}

#[cfg(unix)]
#[test]
fn recovery_opener_rejects_links_fifo_and_root_substitution() {
    use std::os::unix::fs::symlink;

    for kind in ["symlink", "hardlink", "fifo"] {
        let (root, prior, log, event) = initialized(kind);
        publish_interrupted_manually(&root, &log, &event);
        let events = root.path().join("events.jsonl");
        let backup = root.path().join("events.backup");
        fs::rename(&events, &backup).unwrap();
        match kind {
            "symlink" => symlink("events.backup", &events).unwrap(),
            "hardlink" => fs::hard_link(&backup, &events).unwrap(),
            "fifo" => {
                let path = std::ffi::CString::new(events.to_str().unwrap()).unwrap();
                assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
            }
            _ => unreachable!(),
        }
        assert!(
            FileJournal::recover_interrupted_append(
                root.path(),
                &prior,
                &event.event_id,
                &binding(),
            )
            .is_err()
        );
    }

    let (root, prior, log, event) = initialized("root-substitution");
    publish_interrupted_manually(&root, &log, &event);
    let moved = root.path().with_extension("moved");
    fs::rename(root.path(), &moved).unwrap();
    symlink(&moved, root.path()).unwrap();
    assert!(
        FileJournal::recover_interrupted_append(root.path(), &prior, &event.event_id, &binding(),)
            .is_err()
    );
    fs::remove_file(root.path()).unwrap();
    fs::rename(moved, root.path()).unwrap();
}
