use super::super::{
    capture, git_failure_message, git_root_from_stdout, git_root_text, git_status_text,
};
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn status_unavailable_blocks_instead_of_zero_work_pass() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "rust-format-status-unavailable",
    );
    std::fs::create_dir_all(&root).expect("non-git root");
    let captured = capture(&root);
    let stdout = String::from_utf8(captured.stdout).expect("stdout utf8");

    assert_eq!(captured.exit_code, 1);
    assert!(stdout.contains("ultragoal-loop-format-check blocked"));
    assert!(stdout.contains("failure_class=rust_format_status_unavailable"));
    assert!(stdout.contains("where_failed=loop.format_check.changed_inputs"));
    std::fs::remove_dir_all(root).expect("cleanup rust format temp root");
}

#[test]
fn inaccessible_root_blocks_changed_input_discovery() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("rust-format-missing-root");
    let captured = capture(&root);
    let stdout = String::from_utf8(captured.stdout).expect("stdout utf8");

    assert_eq!(captured.exit_code, 1);
    assert!(stdout.contains("ultragoal-loop-format-check blocked"));
    assert!(stdout.contains("routine formatting root is not accessible"));
}

#[test]
fn existing_non_git_root_blocks_changed_input_discovery() {
    let root = std::env::temp_dir().join(format!(
        "ultragoal-rust-format-non-git-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("external temp root");

    let captured = capture(&root);
    let stdout = String::from_utf8(captured.stdout).expect("stdout utf8");

    assert_eq!(captured.exit_code, 1);
    assert!(stdout.contains("ultragoal-loop-format-check blocked"));
    assert!(stdout.contains("routine formatting root is not an exact package git root"));
    std::fs::remove_dir_all(root).expect("cleanup external temp root");
}

#[test]
fn corrupt_git_status_blocks_with_agent_legible_reason() {
    let root = git_fixture("rust-format-corrupt-index");
    write_text(&root.join(".git/index"), "not a git index");

    let captured = capture(&root);
    let stdout = String::from_utf8(captured.stdout).expect("stdout utf8");

    assert_eq!(captured.exit_code, 1);
    assert!(stdout.contains("ultragoal-loop-format-check blocked"));
    assert!(stdout.contains("git status failed while discovering changed Rust files"));
    cleanup(root);
}

#[test]
fn git_failure_messages_cover_empty_and_specific_stderr() {
    assert_eq!(
        git_failure_message(
            b"",
            "git status failed while discovering changed Rust files for routine formatting",
            "git status failed while discovering changed Rust files",
        ),
        "git status failed while discovering changed Rust files for routine formatting"
    );
    assert_eq!(
        git_failure_message(
            b"fatal: not a git repository\n",
            "routine formatting root is not inside a git worktree",
            "routine formatting root is not an exact package git root",
        ),
        "routine formatting root is not an exact package git root: fatal: not a git repository"
    );
}

#[test]
fn git_launch_failures_are_agent_legible_product_blocks() {
    let status_err = git_status_text(Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "git unavailable",
    )))
    .expect_err("git status launch failure");
    assert!(status_err.contains("git status could not launch for routine formatting"));
    assert!(status_err.contains("git unavailable"));

    let root_err = git_root_text(Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "git unavailable",
    )))
    .expect_err("git root launch failure");
    assert!(root_err.contains("git root could not be resolved for routine formatting"));
    assert!(root_err.contains("git unavailable"));
}

#[test]
fn run_returns_the_routine_formatter_exit_code() {
    let root = git_fixture("rust-format-run-clean-root");
    let code = super::super::run(&root).expect("routine formatter run delegates to capture");

    assert_eq!(code, 0);
    cleanup(root);
}

#[test]
fn git_root_stdout_parser_rejects_empty_unreadable_and_missing_roots() {
    assert!(
        git_root_from_stdout(Vec::new())
            .expect_err("empty stdout rejected")
            .contains("git root output was empty")
    );
    assert!(
        git_root_from_stdout(vec![0xff, 0xfe])
            .expect_err("unreadable stdout rejected")
            .contains("git root output was empty")
    );
    assert!(
        git_root_from_stdout(b"/definitely/missing/ultragoal/root\n".to_vec())
            .expect_err("missing root rejected")
            .contains("git root output was empty")
    );
}

#[test]
fn clean_git_root_reports_no_changed_rust_work() {
    let root = git_fixture("rust-format-clean-root");
    let captured = capture(&root);
    let stdout = String::from_utf8(captured.stdout).expect("stdout utf8");

    assert_eq!(captured.exit_code, 0);
    assert!(stdout.contains("ultragoal-loop-format-check pass"));
    assert!(stdout.contains("mode=changed-rust"));
    assert!(stdout.contains("work_unit_count=0"));
    assert!(stdout.contains("no changed Rust source files require routine formatting"));
    cleanup(root);
}

pub(super) fn git_fixture(name: &str) -> PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(name);
    std::fs::create_dir_all(&root).expect("fixture root");
    let output = Command::new("git")
        .arg("init")
        .current_dir(&root)
        .output()
        .expect("git init launches");
    assert!(
        output.status.success(),
        "git init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    root
}

pub(super) fn write_text(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("fixture parent");
    }
    std::fs::write(path, text).expect("fixture file");
}

pub(super) fn cleanup(root: PathBuf) {
    std::fs::remove_dir_all(root).expect("cleanup rust format temp root");
}
