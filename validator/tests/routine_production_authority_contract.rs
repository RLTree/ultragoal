use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_CHILD_REFUSAL: AtomicU64 = AtomicU64::new(0);

#[test]
fn production_source_exposes_no_arbitrary_process_binding_surface() {
    let selection = include_str!("../src/routine_work/runtime_adapter/selection_limit.rs");
    let invocation = include_str!("../src/routine_work/runtime_adapter/invocation_binding.rs");
    let mediation =
        include_str!("../src/routine_work/runtime_adapter/mediator/intent_mediation.rs");
    let process =
        include_str!("../src/routine_work/runtime_adapter/mediator/process/process_execution.rs");
    let launch = include_str!(
        "../src/routine_work/runtime_adapter/mediator/process/darwin_suspended_launch.rs"
    );
    let binding =
        include_str!("../src/routine_work/runtime_adapter/mediator/process/object_bound_launch.rs");
    let custody = include_str!(
        "../src/routine_work/runtime_adapter/mediator/process/spawn_test_observation.rs"
    );
    let settlement =
        include_str!("../src/routine_work/runtime_adapter/mediator/process/custody_settlement.rs");
    let runner = include_str!("../src/routine_work/runtime_adapter/runner_binding.rs");
    let reconciliation =
        include_str!("../src/routine_work/runtime_adapter/execution_reconciliation.rs");
    let grant = include_str!("../src/routine_work/runtime_adapter/mediator/grant_validation.rs");

    assert!(!selection.contains("external-process-exit-v1"));
    assert!(!selection.contains("bind_routine_invocation("));
    assert!(!selection.contains("bind_routine_invocation_with_environment"));
    assert!(selection.contains("bind_rust_source_syntax_invocation("));
    assert!(invocation.contains("RUST_SOURCE_SYNTAX_ARGUMENTS"));
    assert!(invocation.contains("adapter-rust-source-input-missing"));
    assert!(!mediation.contains("framed_input.as_ref()"));
    assert!(!mediation.contains("unwrap_or(\"none\")"));
    assert!(
        mediation.find("validate_rust_source_observation").unwrap()
            < mediation.find("ResultArtifactWire").unwrap()
    );
    assert!(process.contains("framed_input: Vec<u8>"));
    assert!(!process.contains("framed_input: Option"));
    assert!(process.contains("spawn_exact_program"));
    assert!(process.contains("frame_sandboxed_input"));
    assert!(!process.contains("Command::new"));
    assert!(launch.contains("POSIX_SPAWN_START_SUSPENDED"));
    assert!(launch.contains("posix_spawn_file_actions_adddup2"));
    assert!(!launch.contains("/dev/fd"));
    assert!(binding.contains("validate_loaded_executable"));
    assert!(binding.contains("setup.configure"));
    assert!(!custody.contains("impl Drop for SpawnSetupGuard"));
    assert!(!custody.contains("impl Drop for RunningProcess"));
    assert!(settlement.contains("cleanup_owned_process"));
    assert!(reconciliation.contains("adapter-current-runner-substituted"));
    assert!(runner.contains("invocation.environment != expected_environment"));
    assert!(grant.contains("intent.environment() != &expected_environment"));
}

#[test]
fn publication_is_staged_before_cache_and_terminal_settlement() {
    let mediation = include_str!("../src/routine_work/runtime_adapter/mediator/no_op_mediation.rs");
    let output = include_str!(
        "../src/routine_work/runtime_adapter/mediator/filesystem/ownership_rejection.rs"
    );
    let reservation =
        include_str!("../src/routine_work/runtime_adapter/mediator/read_source_binding.rs");
    let lifecycle =
        include_str!("../src/routine_work/runtime_adapter/mediator/reservation_lifecycle.rs");
    let error = include_str!("../src/routine_work/error.rs");
    let stage = mediation.find("attempt.stage_success").unwrap();
    let publish = mediation.find("publisher.publish").unwrap();
    let settle = mediation.find("attempt.settle_success").unwrap();
    assert!(stage < publish && publish < settle);
    assert!(output.contains("mediator-output-scope-not-empty"));
    assert!(output.contains("capture_owned_delta"));
    assert!(output.contains("held != scope.identity"));
    assert!(!reservation.contains("impl Drop for AttemptReservation"));
    assert!(mediation.contains("complete_intent_transition"));
    assert!(mediation.contains("observe_staged_transition(&attempt, || Ok(()))"));
    assert!(!mediation.contains("(Err(_), Err(error))"));
    assert!(lifecycle.contains("attempt.record_failure_and_transition(record)"));
    assert!(!lifecycle.contains("ReservationFailurePanic"));
    assert!(!lifecycle.contains("error.transition_failure()"));
    assert!(!error.contains("reservation_failure_evidence"));
    assert!(!error.contains("with_reservation_failure_evidence"));
    assert!(!error.contains("pub(crate) fn with_transition_failure"));
}

#[test]
fn built_public_child_request_refuses_before_input_and_without_writes() {
    let scratch_root = std::env::var_os("CODEX_WORKTREE_SCRATCH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| panic!("CODEX_WORKTREE_SCRATCH is required"));
    let scratch_root =
        fs::canonicalize(scratch_root).expect("configured worktree scratch is unavailable");
    let scratch = loop {
        let candidate = scratch_root.join(format!(
            "routine-child-refusal-{}-{}",
            std::process::id(),
            NEXT_CHILD_REFUSAL.fetch_add(1, Ordering::Relaxed)
        ));
        match fs::create_dir(&candidate) {
            Ok(()) => break candidate,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => panic!("child refusal fixture claim failed: {error}"),
        }
    };
    let root = scratch.join("root");
    let home = scratch.join("home");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&home).unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_ultragoal"))
        .args(["--json", "check", "routine"])
        .current_dir(&root)
        .env("HOME", &home)
        .env("HUL_ROUTINE_CHILD_FD", "198")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(br#"{"schema_version":"RustSourceSyntaxFrame-v1"}"#);
    }
    let output = child.wait_with_output().unwrap();
    assert_ne!(output.status.code(), Some(0), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("RoutineBehaviorRefusal-v1"));
    assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
    assert_eq!(fs::read_dir(&home).unwrap().count(), 0);
    fs::remove_dir_all(&scratch).unwrap();
}
