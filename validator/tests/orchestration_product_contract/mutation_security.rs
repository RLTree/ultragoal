use crate::orchestration::*;
use crate::orchestration_product::*;
use crate::product_fixture::*;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;

fn running(label: &str) -> (TestRoot, JournalHead) {
    let (root, mut engine) = durable_engine(label, false);
    engine.grant_lease(1, lease(40)).unwrap();
    engine.start(2, "lease-001").unwrap();
    let head = engine.journal_head().unwrap().clone();
    drop(engine);
    (root, head)
}

fn submitted(label: &str) -> (TestRoot, JournalHead, String) {
    let (root, mut engine) = durable_engine(label, false);
    let artifacts = TestRoot::new(&format!("{label}-artifacts"));
    let lease = lease(40);
    let result = worker_result(&artifacts);
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
    let head = engine.journal_head().unwrap().clone();
    drop(engine);
    (root, head, commitment)
}

#[test]
fn unknown_artifact_is_rejected_even_with_a_self_consistent_rewritten_chain() {
    let (root, head, _) = submitted("unknown-artifact");
    let events_path = root.path().join("events.jsonl");
    let mut lines = fs::read(&events_path)
        .unwrap()
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| line.to_vec())
        .collect::<Vec<_>>();
    let mut frame: serde_json::Value = serde_json::from_slice(lines.last().unwrap()).unwrap();
    let mut event: OrchestrationEvent = serde_json::from_value(frame["event"].clone()).unwrap();
    let EventKind::WorkerSubmitted {
        lease_id,
        mut commitment,
        ..
    } = event.event
    else {
        panic!("expected submission event")
    };
    commitment
        .artifact_digests
        .insert("unknown/outside.bin".to_owned(), digest('4'));
    let result_commitment_id = content_digest(&serde_json::to_vec(&commitment).unwrap());
    event = OrchestrationEvent::create(
        event.sequence,
        event.prior_event_id,
        event.binding,
        event.actor,
        event.logical_tick,
        EventKind::WorkerSubmitted {
            lease_id,
            commitment,
            result_commitment_id,
        },
    )
    .unwrap();
    frame["event"] = serde_json::to_value(&event).unwrap();
    lines.pop();
    lines.push(journal_frame_bytes(&event));
    // Restore the canonical JSONL separators after flattening each frame.
    let frames = fs::read(&events_path).unwrap();
    let prefix_len = frames
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .take((head.event_count - 1) as usize)
        .map(|line| line.len() + 1)
        .sum::<usize>();
    let mut log_bytes = frames[..prefix_len].to_vec();
    log_bytes.extend(journal_frame_bytes(&event));
    log_bytes.push(b'\n');
    fs::write(&events_path, &log_bytes).unwrap();
    let mut rewritten_head = head;
    rewritten_head.last_event_id = event.event_id;
    rewritten_head.log_sha256 = format!("sha256:{:x}", Sha256::digest(&log_bytes));
    let mut head_bytes = serde_json::to_vec(&rewritten_head).unwrap();
    head_bytes.push(b'\n');
    fs::write(root.path().join("head.json"), head_bytes).unwrap();

    let workspace = ProductWorkspace::open(root.path()).unwrap();
    assert_eq!(
        query(
            &context(),
            &workspace,
            &QueryRequest {
                expected_head: rewritten_head,
                tick: 4,
                live_workers: BTreeSet::from(["worker-a".to_owned()]),
            },
        )
        .unwrap_err(),
        ProductError::UnknownArtifact
    );
}

#[test]
fn journal_frame_and_artifact_digest_mutation_are_not_receipts() {
    for label in ["journal-frame", "artifact-digest"] {
        let (root, head, _) = submitted(label);
        let workspace = ProductWorkspace::open(root.path()).unwrap();
        let events = root.path().join("events.jsonl");
        let mut bytes = fs::read(&events).unwrap();
        let index = bytes.iter().position(|byte| *byte == b'b').unwrap();
        bytes[index] = b'c';
        fs::write(&events, bytes).unwrap();
        assert!(
            query(
                &context(),
                &workspace,
                &QueryRequest {
                    expected_head: head,
                    tick: 4,
                    live_workers: BTreeSet::from(["worker-a".to_owned()]),
                },
            )
            .is_err()
        );
    }
}

#[cfg(unix)]
#[test]
fn symlink_hardlink_and_special_journal_entries_are_rejected() {
    use std::os::unix::fs::symlink;
    for kind in ["symlink", "hardlink", "fifo"] {
        let (root, head) = running(kind);
        let workspace = ProductWorkspace::open(root.path()).unwrap();
        let events = root.path().join("events.jsonl");
        let backup = root.path().join("events.backup");
        fs::rename(&events, &backup).unwrap();
        match kind {
            "symlink" => symlink("events.backup", &events).unwrap(),
            "hardlink" => fs::hard_link(&backup, &events).unwrap(),
            "fifo" => {
                let value = std::ffi::CString::new(events.to_str().unwrap()).unwrap();
                assert_eq!(unsafe { libc::mkfifo(value.as_ptr(), 0o600) }, 0);
            }
            _ => unreachable!(),
        }
        assert!(
            query(
                &context(),
                &workspace,
                &QueryRequest {
                    expected_head: head,
                    tick: 3,
                    live_workers: BTreeSet::from(["worker-a".to_owned()]),
                },
            )
            .is_err()
        );
    }
}

#[test]
fn ancestor_swap_workspace_swap_and_path_escape_are_rejected() {
    let (root, head) = running("workspace-swap");
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let moved = root.path().with_extension("moved");
    fs::rename(root.path(), &moved).unwrap();
    fs::create_dir(root.path()).unwrap();
    for name in ["events.jsonl", "head.json", "journal.lock"] {
        fs::copy(moved.join(name), root.path().join(name)).unwrap();
    }
    assert_eq!(
        query(
            &context(),
            &workspace,
            &QueryRequest {
                expected_head: head,
                tick: 3,
                live_workers: BTreeSet::from(["worker-a".to_owned()]),
            },
        )
        .unwrap_err(),
        ProductError::WorkspaceChanged
    );
    fs::remove_dir_all(root.path()).unwrap();
    fs::rename(&moved, root.path()).unwrap();
    let escaped = root.path().join("child").join("..");
    assert_eq!(
        ProductWorkspace::open(escaped).unwrap_err(),
        ProductError::InvalidWorkspacePath
    );
}
