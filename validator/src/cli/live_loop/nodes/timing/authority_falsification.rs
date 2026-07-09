use serde_json::json;

#[path = "test_rows.rs"]
mod test_rows;
use self::test_rows::current_timing_row;

#[test]
fn command_identity_rejects_broad_sweep_or_or_filter() {
    let candidate = "sha256:current";
    let changed = crate::digest::bytes(b"");
    let context = crate::digest::bytes(
        format!(
            "validator={};law={};schema={};fixture={};tier=hot;cache=verified-local",
            crate::cli::live_loop::graph::validator_version(),
            crate::cli::live_loop::graph::law_version(),
            crate::cli::live_loop::graph::schema_version(),
            crate::cli::live_loop::graph::fixture_version()
        )
        .as_bytes(),
    );
    let input = super::graph::surface_input_digest(
        super::surface_by_id("fmt_check").expect("fmt surface"),
        candidate,
        &changed,
        &context,
    );
    let mut row = current_timing_row(candidate, &input, "pass", "none");
    let expected = vec![
        "cargo".to_string(),
        "fmt".to_string(),
        "--all".to_string(),
        "--check".to_string(),
    ];
    assert!(super::row_authority::command_identity_matches(
        &row,
        "cargo fmt --all --check",
        &expected
    ));
    row.as_object_mut().expect("row object").insert(
        "command_argv".to_string(),
        json!(["cargo", "test", "--offline"]),
    );
    assert!(!super::row_authority::command_identity_matches(
        &row,
        "cargo fmt --all --check",
        &expected
    ));
    row.as_object_mut().expect("row object").insert(
        "command_argv".to_string(),
        json!(["cargo", "test", "--offline", "live_loop|measurement"]),
    );
    assert!(!super::row_authority::command_identity_matches(
        &row,
        "cargo fmt --all --check",
        &expected
    ));
}

#[cfg(unix)]
#[test]
fn command_observation_path_rejects_symlink_parent_traversal() {
    use std::os::unix::fs::symlink;

    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "live-loop-timing-symlink-receipt",
    );
    let private = root.join("private");
    std::fs::create_dir_all(&private).expect("private dir");
    let commands = root.join("validation_artifacts/observability/live-loop/commands");
    std::fs::create_dir_all(commands.parent().expect("commands parent")).expect("parent");
    symlink(&private, &commands).expect("symlink commands");
    assert!(
        super::receipt_path::observation_receipt_path_for_tests(
            &root,
            "validation_artifacts/observability/live-loop/commands/fmt_check-command-observation.json",
        )
        .is_none()
    );
    std::fs::remove_dir_all(root).expect("cleanup symlink receipt");
}
