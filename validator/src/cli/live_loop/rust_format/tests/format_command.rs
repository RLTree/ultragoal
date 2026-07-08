use super::super::capture;
use super::git_workspace::{cleanup, git_fixture, write_text};

#[test]
fn changed_rust_sources_run_rustfmt_check() {
    let root = git_fixture("rust-format-changed-source-pass");
    write_text(
        &root.join("validator/src/lib.rs"),
        "pub fn answer() -> i32 {\n    42\n}\n",
    );

    let captured = capture(&root);
    let stdout = String::from_utf8(captured.stdout).expect("stdout utf8");

    assert_eq!(captured.exit_code, 0);
    assert!(stdout.contains("ultragoal-loop-format-check pass"));
    assert!(stdout.contains("mode=changed-rust"));
    assert!(stdout.contains("work_unit_count=1"));
    assert!(stdout.contains("rust formatting check passed for the routine affected set"));
    cleanup(root);
}

#[test]
fn changed_rust_format_failure_is_agent_legible() {
    let root = git_fixture("rust-format-changed-source-fail");
    write_text(
        &root.join("validator/src/lib.rs"),
        "pub fn answer()->i32{42}\n",
    );

    let captured = capture(&root);
    let stdout = String::from_utf8(captured.stdout).expect("stdout utf8");

    assert_eq!(captured.exit_code, 1);
    assert!(stdout.contains("ultragoal-loop-format-check fail"));
    assert!(stdout.contains("failure_class=rust_format_changed_file_failed"));
    assert!(stdout.contains("where_failed=loop.format_check.changed_rust"));
    assert!(stdout.contains("routine Rust formatting command exited"));
    assert!(stdout.contains("run rustfmt on the changed Rust source files"));
    cleanup(root);
}

#[test]
fn rustfmt_config_change_runs_workspace_format_check() {
    let root = git_fixture("rust-format-workspace-config");
    write_text(
        &root.join("Cargo.toml"),
        "[package]\nname = \"rust-format-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n\n[workspace]\n",
    );
    write_text(
        &root.join("src/lib.rs"),
        "pub fn answer() -> i32 {\n    42\n}\n",
    );
    write_text(&root.join("rustfmt.toml"), "edition = \"2024\"\n");

    let captured = capture(&root);
    let stdout = String::from_utf8(captured.stdout).expect("stdout utf8");

    assert_eq!(captured.exit_code, 0);
    assert!(stdout.contains("ultragoal-loop-format-check pass"));
    assert!(stdout.contains("mode=workspace-config"));
    assert!(stdout.contains("work_unit_count=1"));
    cleanup(root);
}
