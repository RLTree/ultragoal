use super::{CapturedOutput, OutputBudget, PendingOutput, observe};
use sha2::{Digest, Sha256};
use std::io::{Cursor, Read};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};

struct PrefixThenGatedEof {
    prefix: Option<Vec<u8>>,
    waiting: SyncSender<()>,
    release: Receiver<()>,
}

impl Read for PrefixThenGatedEof {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if let Some(prefix) = self.prefix.take() {
            buffer[..prefix.len()].copy_from_slice(&prefix);
            return Ok(prefix.len());
        }
        self.waiting.send(()).unwrap();
        self.release.recv().unwrap();
        Ok(0)
    }
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn assert_absent(output: &CapturedOutput, needles: &[&[u8]]) {
    let json = serde_json::to_vec(output).unwrap();
    let debug = format!("{output:?}");
    let text = String::from_utf8_lossy(&json);
    for bytes in needles.iter().copied().filter(|bytes| !bytes.is_empty()) {
        assert!(!json.windows(bytes.len()).any(|seen| seen == bytes));
        let digest = digest(bytes);
        let bare = digest.strip_prefix("sha256:").unwrap();
        assert!(!text.contains(&digest) && !text.contains(bare));
        assert!(!debug.contains(&digest) && !debug.contains(bare));
        let numeric = format!(
            "[{}]",
            bytes
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(",")
        );
        assert!(!text.contains(&numeric) && !debug.contains(&numeric));
    }
}

fn assert_withheld(output: &CapturedOutput, needles: &[&[u8]]) {
    assert!(output.retained().is_empty());
    assert_eq!(output.captured_byte_length(), 0);
    assert_eq!(output.captured_sha256, digest(&[]));
    assert!(output.truncated);
    assert!(!output.observation_limit_exceeded);
    assert!(!output.captured_byte_length_is_lower_bound);
    assert_absent(output, needles);
}

fn orchestrate(
    prefix: &[u8],
    secrets: &[Vec<u8>],
    prefix_is_first: bool,
) -> (Arc<OutputBudget>, PendingOutput, PendingOutput) {
    let budget = Arc::new(OutputBudget::for_bound_secrets(prefix.len() + 2, secrets));
    let (waiting_tx, waiting_rx) = sync_channel(0);
    let (release_tx, release_rx) = sync_channel(0);
    let prefix_budget = Arc::clone(&budget);
    let prefix = prefix.to_vec();
    let prefix_reader = std::thread::spawn(move || {
        observe(
            PrefixThenGatedEof {
                prefix: Some(prefix),
                waiting: waiting_tx,
                release: release_rx,
            },
            8192,
            &prefix_budget,
        )
        .unwrap()
    });
    waiting_rx.recv().unwrap();
    let overflow = observe(Cursor::new(b"xxx"), 8192, &budget).unwrap();
    assert!(budget.exceeded());
    assert_eq!(budget.observed(), budget.limit);
    release_tx.send(()).unwrap();
    let prefix = prefix_reader.join().unwrap();
    if prefix_is_first {
        (budget, prefix, overflow)
    } else {
        (budget, overflow, prefix)
    }
}

fn finalize_in_order(
    budget: &OutputBudget,
    first: PendingOutput,
    second: PendingOutput,
    reverse: bool,
) -> (CapturedOutput, CapturedOutput) {
    if reverse {
        let outputs = budget.finalize_streams(second, first);
        assert!(outputs.output_limit_exceeded);
        (outputs.second, outputs.first)
    } else {
        let outputs = budget.finalize_streams(first, second);
        assert!(outputs.output_limit_exceeded);
        (outputs.first, outputs.second)
    }
}

#[test]
fn classified_sensitive_streams_stay_withheld_across_shared_overflow_and_order() {
    let secret = b"123456".to_vec();
    let transformed = b"1e240";
    for prefix_is_first in [false, true] {
        for reverse in [false, true] {
            let (budget, first, second) =
                orchestrate(transformed, std::slice::from_ref(&secret), prefix_is_first);
            let (first, second) = finalize_in_order(&budget, first, second, reverse);
            for output in [&first, &second] {
                assert!(!output.observation_limit_exceeded);
                assert_withheld(output, &[&secret, transformed]);
            }
        }
    }
}

#[test]
fn public_output_preserves_exact_shared_and_local_budget_semantics() {
    let exact_budget = OutputBudget::new(6);
    let first = observe(Cursor::new(b"abc"), 8192, &exact_budget).unwrap();
    let second = observe(Cursor::new(b"def"), 8192, &exact_budget).unwrap();
    let exact = exact_budget.finalize_streams(first, second);
    assert_eq!(exact.first.retained(), b"abc");
    assert_eq!(exact.second.retained(), b"def");
    assert!(!exact.output_limit_exceeded);

    let local_budget = OutputBudget::new(32);
    let retained = observe(Cursor::new(b"ordinary-output"), 4, &local_budget).unwrap();
    let empty = observe(Cursor::new([]), 4, &local_budget).unwrap();
    let local = local_budget.finalize_streams(retained, empty).first;
    assert_eq!(local.retained(), b"ordi");
    assert_eq!(local.captured_byte_length(), 15);
    assert_eq!(local.captured_sha256, digest(b"ordinary-output"));
    assert!(local.truncated && !local.observation_limit_exceeded);
    assert!(!local.captured_byte_length_is_lower_bound);

    let global_budget = OutputBudget::new(3);
    let global = observe(Cursor::new(b"abcdef"), 8192, &global_budget).unwrap();
    let empty = observe(Cursor::new([]), 8192, &global_budget).unwrap();
    let global = global_budget.finalize_streams(global, empty).first;
    assert_eq!(global.retained(), b"abc");
    assert!(global.observation_limit_exceeded);
    assert!(global.captured_byte_length_is_lower_bound);
}

#[test]
fn sensitive_zero_one_exact_and_maximum_boundaries_never_publish_content() {
    let secret = vec![b'z'; 4096];
    for (input, limit) in [
        (secret.as_slice(), 0),
        (&secret[..1], 1),
        (secret.as_slice(), secret.len()),
        (secret.as_slice(), secret.len() - 1),
    ] {
        let budget = OutputBudget::for_bound_secrets(limit, std::slice::from_ref(&secret));
        let output = observe(Cursor::new(input), 8192, &budget).unwrap();
        let empty = observe(Cursor::new([]), 8192, &budget).unwrap();
        let output = budget.finalize_streams(output, empty).first;
        assert_withheld(&output, &[&secret, input]);
        assert!(!output.observation_limit_exceeded);
    }
}

#[cfg(target_os = "macos")]
#[test]
fn sensitive_pair_is_absent_from_canonical_run_json() {
    use crate::context::{BuildRequest, LiveContext};
    use std::fs;
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "ultragoal-sensitive-output-run-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    fs::create_dir_all(&root).unwrap();
    assert!(
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(&root)
            .status()
            .unwrap()
            .success()
    );
    let context = LiveContext::build(
        BuildRequest::new(&root)
            .expect_repository_root(&root)
            .expect_worktree_root(&root)
            .probe_tool("sandbox-exec")
            .probe_tool("true"),
    )
    .unwrap();
    let secret = b"123456".to_vec();
    let transformed = b"1e240";
    let (budget, stdout, stderr) = orchestrate(transformed, std::slice::from_ref(&secret), true);
    let outputs = budget.finalize_streams(stdout, stderr);
    let mut run =
        super::super::spec::CommandSpec::catalog_read("sensitive-output-canonical", "true")
            .run(&context)
            .unwrap();
    run.install_output_limit_for_test(outputs.first, outputs.second);
    let json = run.to_canonical_json().unwrap();
    let debug = format!("{run:?}");
    for bytes in [&secret[..], transformed] {
        assert!(!json.windows(bytes.len()).any(|seen| seen == bytes));
        assert!(
            !debug
                .as_bytes()
                .windows(bytes.len())
                .any(|seen| seen == bytes)
        );
        let raw_digest = digest(bytes);
        let bare = raw_digest.strip_prefix("sha256:").unwrap();
        for surface in [String::from_utf8_lossy(&json).as_ref(), debug.as_str()] {
            assert!(!surface.contains(&raw_digest) && !surface.contains(bare));
        }
    }
    fs::remove_dir_all(root).unwrap();
}
