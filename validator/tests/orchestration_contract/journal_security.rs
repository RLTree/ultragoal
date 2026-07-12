use crate::durable_support::*;
use crate::orchestration::*;
use crate::support::*;
use sha2::{Digest, Sha256};
use std::fs;

fn populated(label: &str) -> (JournalRoot, JournalHead, EventLog) {
    let (mut engine, _, root) = durable_engine(label);
    engine.grant_lease(1, lease()).unwrap();
    engine.start(2, "lease-001").unwrap();
    let head = engine.journal_head().unwrap().clone();
    let log = engine.event_log();
    drop(engine);
    (root, head, log)
}

fn event_lines(root: &JournalRoot) -> Vec<Vec<u8>> {
    fs::read(root.path().join("events.jsonl"))
        .unwrap()
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| line.to_vec())
        .collect()
}

fn write_lines_and_head(root: &JournalRoot, lines: &[Vec<u8>]) -> JournalHead {
    let mut bytes = lines.join(&b'\n');
    bytes.push(b'\n');
    let last: serde_json::Value = serde_json::from_slice(lines.last().unwrap()).unwrap();
    let head = JournalHead {
        schema_version: "OrchestrationJournalHead-v1".to_owned(),
        binding: binding(),
        event_count: lines.len() as u64,
        last_event_id: last["event"]["event_id"].as_str().unwrap().to_owned(),
        log_sha256: format!("sha256:{:x}", Sha256::digest(&bytes)),
    };
    fs::write(root.path().join("events.jsonl"), bytes).unwrap();
    let mut head_bytes = serde_json::to_vec(&head).unwrap();
    head_bytes.push(b'\n');
    fs::write(root.path().join("head.json"), head_bytes).unwrap();
    head
}

#[test]
fn truncated_unknown_duplicate_and_reordered_frames_fail_closed() {
    let (truncated, _, _) = populated("truncated");
    let mut bytes = fs::read(truncated.path().join("events.jsonl")).unwrap();
    bytes.pop();
    fs::write(truncated.path().join("events.jsonl"), bytes).unwrap();
    assert_eq!(
        FileJournal::open(truncated.path()).unwrap_err(),
        OrchestrationError::JournalCorrupt
    );

    let (unknown, _, _) = populated("unknown-frame");
    let mut lines = event_lines(&unknown);
    let mut row: serde_json::Value = serde_json::from_slice(&lines[1]).unwrap();
    row["unknown"] = serde_json::json!(true);
    lines[1] = serde_json::to_vec(&row).unwrap();
    write_lines_and_head(&unknown, &lines);
    assert_eq!(
        FileJournal::open(unknown.path()).unwrap_err(),
        OrchestrationError::JournalCorrupt
    );

    for (label, mutate) in [("duplicate", 0_u8), ("reordered", 1_u8)] {
        let (root_dir, _, _) = populated(label);
        let mut lines = event_lines(&root_dir);
        if mutate == 0 {
            lines.insert(1, lines[1].clone());
        } else {
            lines.swap(1, 2);
        }
        write_lines_and_head(&root_dir, &lines);
        assert_eq!(
            FileJournal::open(root_dir.path()).unwrap_err(),
            OrchestrationError::JournalCorrupt
        );
    }
}

#[cfg(unix)]
#[test]
fn symlink_hardlink_fifo_and_root_substitution_are_rejected() {
    use std::os::unix::fs::symlink;

    let (symlink_root, _, _) = populated("symlink-file");
    let events = symlink_root.path().join("events.jsonl");
    let real = symlink_root.path().join("events.real");
    fs::rename(&events, &real).unwrap();
    symlink(&real, &events).unwrap();
    assert!(FileJournal::open(symlink_root.path()).is_err());

    let (hardlink_root, _, _) = populated("hardlink-file");
    fs::hard_link(
        hardlink_root.path().join("events.jsonl"),
        hardlink_root.path().join("events.alias"),
    )
    .unwrap();
    assert!(FileJournal::open(hardlink_root.path()).is_err());

    let (fifo_root, _, _) = populated("fifo-file");
    let events = fifo_root.path().join("events.jsonl");
    fs::rename(&events, fifo_root.path().join("events.real")).unwrap();
    let fifo = std::ffi::CString::new(events.to_str().unwrap()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    assert!(FileJournal::open(fifo_root.path()).is_err());

    let (swap_root, _, _) = populated("root-swap");
    let journal = FileJournal::open(swap_root.path()).unwrap();
    let moved = swap_root.path().with_extension("moved");
    fs::rename(swap_root.path(), &moved).unwrap();
    fs::create_dir(swap_root.path()).unwrap();
    assert_eq!(
        journal.inspect().unwrap_err(),
        OrchestrationError::JournalCorrupt
    );
    fs::remove_dir(swap_root.path()).unwrap();
    fs::rename(moved, swap_root.path()).unwrap();
}

#[test]
fn explicit_repair_accepts_only_one_exact_interrupted_append() {
    let (root_dir, prior_head, prior_log) = populated("repair-one-append");
    let event = prior_log
        .next(
            &binding(),
            worker("worker-a"),
            3,
            EventKind::Heartbeat {
                lease_id: "lease-001".to_owned(),
            },
        )
        .unwrap();
    let mut bytes = fs::read(root_dir.path().join("events.jsonl")).unwrap();
    bytes.extend(b"{\"schema_version\":\"OrchestrationJournalFrame-v1\",\"event\":");
    bytes.extend(serde_json::to_vec(&event).unwrap());
    bytes.extend(b"}\n");
    fs::write(root_dir.path().join("events.jsonl"), bytes).unwrap();
    let repaired = FileJournal::recover_interrupted_append(
        root_dir.path(),
        &prior_head,
        &event.event_id,
        &binding(),
    )
    .unwrap();
    assert_eq!(repaired.head.event_count, prior_head.event_count + 1);

    let (sink, _) = CountingSink::new();
    Orchestrator::restart_durable(
        graph_one(),
        policy(),
        repaired.head,
        root(),
        root_dir.path(),
        sink,
    )
    .unwrap();
    assert_eq!(
        FileJournal::recover_interrupted_append(
            root_dir.path(),
            &prior_head,
            &digest('8'),
            &binding(),
        )
        .unwrap_err(),
        OrchestrationError::JournalConflict
    );
}
