use super::full_command as subject;
use crate::cli::live_loop::surfaces::surface_by_id;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .expect("env lock")
}

#[test]
fn ultragoal_self_calls_use_running_binary_not_target_root_artifact() {
    let runtime = subject::runtime_command_text("target/debug/ultragoal --root . line-caps check");
    assert!(!runtime.starts_with("target/debug/ultragoal "), "{runtime}");
    assert!(runtime.ends_with(" --root . line-caps check"), "{runtime}");
}

#[test]
fn non_ultragoal_commands_stay_literal() {
    assert_eq!(
        subject::runtime_command_text("cargo fmt --all --check"),
        "cargo fmt --all --check"
    );
}

#[test]
fn runtime_ultragoal_command_uses_explicit_cargo_binary_when_available() {
    let _guard = env_lock();
    let previous = std::env::var_os("CARGO_BIN_EXE_ultragoal");
    // SAFETY: this test holds a process-local mutex for the full mutation window.
    unsafe {
        std::env::set_var("CARGO_BIN_EXE_ultragoal", "/tmp/ultragoal product binary");
    }

    let runtime = subject::runtime_command_text("target/debug/ultragoal --root . package digest");

    assert_eq!(
        runtime,
        "'/tmp/ultragoal product binary' --root . package digest"
    );
    restore_runtime_binary_env(previous);
}

#[test]
fn runtime_ultragoal_command_uses_running_ultragoal_binary_when_named_as_product() {
    let runtime = subject::runtime_command_text_with_executable_context(
        "target/debug/ultragoal --root . package digest",
        None,
        Some(PathBuf::from("/tmp/current/ultragoal")),
    );

    assert_eq!(runtime, "'/tmp/current/ultragoal' --root . package digest");
}

#[test]
fn runtime_ultragoal_command_uses_adjacent_product_binary_for_cargo_test_executables() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("runtime-adjacent-binary");
    let deps = root.join("target/debug/deps");
    std::fs::create_dir_all(&deps).expect("deps");
    let product_binary = root.join("target/debug/ultragoal");
    std::fs::write(&product_binary, "").expect("product binary");
    let current = deps.join("validator_tests-abc123");

    let runtime = subject::runtime_command_text_with_executable_context(
        "target/debug/ultragoal --root . package digest",
        None,
        Some(current),
    );

    assert_eq!(
        runtime,
        format!("'{}' --root . package digest", product_binary.display())
    );
    std::fs::remove_dir_all(root).expect("cleanup adjacent binary");
}

#[test]
fn runtime_ultragoal_command_falls_back_to_current_executable_when_adjacent_binary_is_absent() {
    let current = PathBuf::from("/tmp/target/debug/deps/validator_tests-abc123");

    let runtime = subject::runtime_command_text_with_executable_context(
        "target/debug/ultragoal --root . package digest",
        None,
        Some(current.clone()),
    );

    assert_eq!(
        runtime,
        format!("'{}' --root . package digest", current.display())
    );
}

#[test]
fn runtime_ultragoal_command_stays_literal_when_executable_context_is_unavailable() {
    let command = "target/debug/ultragoal --root . package digest";

    let runtime = subject::runtime_command_text_with_executable_context(command, None, None);

    assert_eq!(runtime, command);
}

#[test]
fn runtime_shell_argv_preserves_the_product_command_shape() {
    let argv = subject::runtime_shell_argv("cargo fmt --all --check");

    assert_eq!(argv, ["bash", "-lc", "cargo fmt --all --check"]);
}

#[test]
fn command_launch_failure_is_reported_as_validation_result_not_panic() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("live-loop-command-launch");
    std::fs::create_dir_all(&root).expect("root");
    let surface = surface_by_id("fmt_check").expect("fmt surface");

    let run = subject::run_full_command_with_shell(&root, surface, "/missing/ultragoal-shell");

    assert_eq!(run.exit_code, 1);
    assert!(!run.status_success);
    assert!(run.launch_error);
    assert!(run.duration_ms >= 1);
    assert!(run.stdout_digest.starts_with("sha256:"));
    assert!(run.stderr_digest.starts_with("sha256:"));
    std::fs::remove_dir_all(root).expect("cleanup launch failure");
}

#[test]
fn full_and_narrow_command_runners_execute_product_surface_commands() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("live-loop-command-runners");
    std::fs::create_dir_all(&root).expect("root");
    let surface = crate::cli::live_loop::surfaces::LoopValidationSurface {
        id: "unit_command_runner",
        surface: "rust_validation",
        command: "command runner contract",
        canonical_full_command: "printf full-command",
        narrow_rerun: "printf narrow-command",
        telemetry_reconciliation_state: "requires_command_telemetry_roundtrip",
        execution_task_class: crate::scheduler::TaskClass::PureReadParallel,
        execution_serial_reason: "none",
        high_frequency: true,
        hot_loop_policy: "routine_hot_repair",
    };

    let full = subject::run_full_command(&root, surface);
    let narrow = subject::run_narrow_command(&root, surface);

    assert!(full.status_success);
    assert!(narrow.status_success);
    assert_ne!(full.stdout_digest, narrow.stdout_digest);
    std::fs::remove_dir_all(root).expect("cleanup command runners");
}

#[test]
fn ultragoal_runtime_falls_back_to_current_test_binary_when_product_binary_is_unavailable() {
    let _guard = env_lock();
    let previous = std::env::var_os("CARGO_BIN_EXE_ultragoal");
    // SAFETY: this test holds a process-local mutex for the full mutation window.
    unsafe {
        std::env::remove_var("CARGO_BIN_EXE_ultragoal");
    }

    let runtime = subject::runtime_command_text("target/debug/ultragoal --root . package digest");

    assert!(runtime.ends_with(" --root . package digest"), "{runtime}");
    assert!(!runtime.starts_with("target/debug/ultragoal "), "{runtime}");
    restore_runtime_binary_env(previous);
}

fn restore_runtime_binary_env(previous: Option<std::ffi::OsString>) {
    match previous {
        Some(value) => {
            // SAFETY: callers hold the process-local mutex for the full mutation window.
            unsafe { std::env::set_var("CARGO_BIN_EXE_ultragoal", value) }
        }
        None => {
            // SAFETY: callers hold the process-local mutex for the full mutation window.
            unsafe { std::env::remove_var("CARGO_BIN_EXE_ultragoal") }
        }
    }
}
