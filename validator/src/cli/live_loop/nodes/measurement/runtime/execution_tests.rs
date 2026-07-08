use super::super::full_command as subject;
use crate::cli::live_loop::surfaces::LoopValidationSurface;

#[test]
fn direct_command_launch_failure_is_reported_as_validation_result_not_panic() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "live-loop-direct-command-launch",
    );
    std::fs::create_dir_all(&root).expect("root");
    let surface = test_surface(
        "unit_direct_launch_failure",
        "rust_validation",
        "/missing/live-loop-command",
        "/missing/live-loop-command",
    );

    let run = subject::run_full_command(&root, surface);

    assert_eq!(run.exit_code, 1);
    assert!(!run.status_success);
    assert!(run.launch_error);
    assert!(run.duration_ms >= 1);
    assert!(run.stderr_digest.starts_with("sha256:"));
    std::fs::remove_dir_all(root).expect("cleanup direct launch failure");
}

#[test]
fn full_and_narrow_command_runners_execute_product_surface_commands() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("live-loop-command-runners");
    std::fs::create_dir_all(&root).expect("root");
    let surface = test_surface(
        "unit_command_runner",
        "rust_validation",
        "printf full-command",
        "printf narrow-command",
    );

    let full = subject::run_full_command(&root, surface);
    let narrow = subject::run_narrow_command(&root, surface);

    assert!(full.status_success);
    assert!(narrow.status_success);
    assert_ne!(full.stdout_digest, narrow.stdout_digest);
    std::fs::remove_dir_all(root).expect("cleanup command runners");
}

#[test]
fn shell_syntax_command_runner_executes_product_surface_command() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("live-loop-shell-command");
    std::fs::create_dir_all(&root).expect("root");
    let surface = test_surface(
        "unit_shell_command_runner",
        "shell_validation",
        "printf shell-command && printf done",
        "printf shell-rerun && printf done",
    );

    let full = subject::run_full_command(&root, surface);
    let narrow = subject::run_narrow_command(&root, surface);

    assert!(full.status_success);
    assert!(narrow.status_success);
    assert_ne!(full.stdout_digest, narrow.stdout_digest);
    std::fs::remove_dir_all(root).expect("cleanup shell command");
}

fn test_surface(
    id: &'static str,
    surface: &'static str,
    canonical_full_command: &'static str,
    narrow_rerun: &'static str,
) -> LoopValidationSurface {
    LoopValidationSurface {
        id,
        surface,
        command: "command runner contract",
        canonical_full_command,
        narrow_rerun,
        telemetry_reconciliation_state: "requires_command_telemetry_roundtrip",
        execution_task_class: crate::scheduler::TaskClass::PureReadParallel,
        execution_serial_reason: "none",
        high_frequency: true,
        hot_loop_policy: "routine_hot_repair",
    }
}
