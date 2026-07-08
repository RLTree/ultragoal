use super::super::full_command as subject;
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
fn non_ultragoal_command_text_stays_literal() {
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
fn runtime_ultragoal_command_uses_current_executable_when_adjacent_binary_is_absent() {
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
fn runtime_ultragoal_argv_stays_literal_when_executable_context_is_unavailable() {
    let argv = subject::runtime_command_argv_with_executable_context(
        "target/debug/ultragoal --root . package digest",
        None,
        None,
    );

    assert_eq!(
        argv,
        ["target/debug/ultragoal", "--root", ".", "package", "digest"]
    );
}

#[test]
fn runtime_command_argv_uses_direct_product_command_shape() {
    let argv = subject::runtime_command_argv(
        "target/debug/ultragoal --root . loop format check --changed-rust",
    );

    assert_ne!(
        argv[0], "bash",
        "simple product command avoids shell wrapper"
    );
    assert!(argv.iter().any(|arg| arg == "loop"));
    assert!(argv.iter().any(|arg| arg == "--changed-rust"));
}

#[test]
fn product_command_argv_uses_bounded_product_identity_for_receipts() {
    let argv = subject::product_command_argv("target/debug/ultragoal --root . package digest");

    assert_eq!(argv, ["ultragoal", "--root", ".", "package", "digest"]);
    assert_eq!(
        subject::product_command_text("target/debug/ultragoal --root . package digest"),
        "ultragoal --root . package digest"
    );
}

#[test]
fn product_command_argv_preserves_non_ultragoal_surface_identity() {
    let argv = subject::product_command_argv("git status --short --untracked-files=all");

    assert_eq!(argv, ["git", "status", "--short", "--untracked-files=all"]);
}

#[test]
fn cargo_toolchain_commands_execute_through_developer_shell() {
    let argv = subject::runtime_command_argv("cargo fmt --all --check");

    assert_eq!(argv, ["bash", "-lc", "cargo fmt --all --check"]);
    assert_eq!(
        subject::product_command_argv("cargo fmt --all --check"),
        ["cargo", "fmt", "--all", "--check"],
        "receipts keep the product tool surface, while runtime argv records shell resolution"
    );
}

#[test]
fn bare_ultragoal_commands_execute_through_developer_shell() {
    let argv = subject::runtime_command_argv("ultragoal loop run --tier hot");

    assert_eq!(argv, ["bash", "-lc", "ultragoal loop run --tier hot"]);
    assert_eq!(
        subject::product_command_argv("ultragoal loop run --tier hot"),
        ["ultragoal", "loop", "run", "--tier", "hot"],
        "receipts keep the canonical CLI surface while runtime argv records shell resolution"
    );
}

#[test]
fn ordinary_read_commands_execute_directly() {
    let argv = subject::runtime_command_argv("git status --short --untracked-files=all");

    assert_eq!(argv, ["git", "status", "--short", "--untracked-files=all"]);
}

#[test]
fn runtime_command_argv_uses_shell_only_for_shell_specific_syntax() {
    let argv = subject::runtime_command_argv("printf ok > artifact.txt");

    assert_eq!(argv[0], "bash");
    assert_eq!(argv[1], "-lc");
    assert_eq!(argv[2], "printf ok > artifact.txt");
}

#[test]
fn ultragoal_runtime_uses_current_test_binary_when_product_binary_is_unavailable() {
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
