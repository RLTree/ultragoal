use super::super::{FormatResult, command_result, is_rust_source, is_rustfmt_config};

#[test]
fn formatter_launch_failure_has_specific_repair_output() {
    let result = command_result(
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "missing formatter",
        )),
        "changed-rust",
        1,
        7,
        "rust_format_changed_file_failed",
        "loop.format_check.changed_rust",
        "run rustfmt on changed files",
    );
    let line = result.stdout_line();

    assert_eq!(result.exit_code, 1);
    assert!(line.contains("ultragoal-loop-format-check fail"));
    assert!(line.contains("failure_class=rust_format_command_launch_failed"));
    assert!(line.contains("where_failed=loop.format_check.changed_rust"));
    assert!(line.contains("make rustfmt/cargo fmt available"));
    assert!(line.contains("stderr_digest="));
}

#[test]
fn rust_source_path_covers_package_owned_rust_files() {
    assert!(is_rust_source(std::path::Path::new("validator/src/lib.rs")));
    assert!(is_rust_source(std::path::Path::new(
        "validator/tests/cli_surface.rs"
    )));
    assert!(!is_rust_source(std::path::Path::new(
        "validator/src/lib.txt"
    )));
    assert!(!is_rust_source(std::path::Path::new(
        "docs/rust_example.rs"
    )));
    assert!(!is_rustfmt_config(std::path::Path::new(
        "validator/rustfmt.toml.bak"
    )));
}

#[test]
fn pass_output_names_claim_ceiling_and_work_units() {
    let line = FormatResult::pass("changed-rust", 2, 7, "ok").stdout_line();
    assert!(line.contains("ultragoal-loop-format-check pass"));
    assert!(line.contains("work_unit_count=2"));
    assert!(line.contains("why_failed='none'"));
    assert!(line.contains("validation_summary='ok'"));
    assert!(line.contains("claim_ceiling='source-local routine repair only"));
}
