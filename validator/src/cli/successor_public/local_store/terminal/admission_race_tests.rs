use super::append::append_routine_terminal_with_hook;
use super::observations::{RoutineTerminalEvent, terminal_event_id};
use crate::context::{BuildRequest, LiveContext};
use crate::routine_work::{RoutineBinding, RoutineTerminalOutcome};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn append_revalidates_after_admission_before_store_creation() {
    let root = fixture_root();
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join(".gitignore"), b"validation_artifacts/\n").unwrap();
    fs::write(root.join("source.rs"), b"fn main() {}\n").unwrap();
    git(&root, &["init", "--quiet"]);
    git(
        &root,
        &["config", "user.email", "routine-test@example.invalid"],
    );
    git(&root, &["config", "user.name", "Routine Test"]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "--quiet", "-m", "fixture"]);

    let context = LiveContext::build(BuildRequest::new(&root)).unwrap();
    let binding = RoutineBinding::from_live(&context).unwrap();
    let continuation = "routine-cont-admission-race";
    let head = "sha256:terminal-head";
    let event_id = terminal_event_id(continuation, head);
    let terminal = RoutineTerminalEvent {
        event_id: &event_id,
        continuation_id: continuation,
        terminal_ledger_head: head,
        observed_at_unix_ms: 1,
        sequence: 1,
        parent_event_id: None,
        status: "pass",
        transition: "executed",
        terminal_outcome: RoutineTerminalOutcome::Complete,
        finding_binding: None,
    };
    let mut mutate_after_admission = || {
        fs::write(root.join(".gitignore"), b"target/\n").unwrap();
    };

    let result = append_routine_terminal_with_hook(
        &root,
        &context,
        &binding,
        terminal,
        Some(&mut mutate_after_admission),
    );

    assert!(result.is_err());
    assert!(!root.join("validation_artifacts").exists());
    assert!(
        !root
            .join("validation_artifacts/observability/spool/successor-events.jsonl")
            .exists()
    );
    fs::remove_dir_all(root).unwrap();
}

fn fixture_root() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "ultragoal-terminal-admission-race-{}",
        std::process::id()
    ));
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }
    root
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}: {output:?}");
}
