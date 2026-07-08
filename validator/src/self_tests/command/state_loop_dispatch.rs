use serde_json::json;
use std::path::{Path, PathBuf};

fn write_json(path: &Path, value: &serde_json::Value) {
    let parent = path.parent().expect("json fixture path has a parent");
    std::fs::create_dir_all(parent).expect("parent");
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn args(root: PathBuf, raw: &[&str]) -> crate::Args {
    crate::Args {
        root,
        command: crate::parse_command(&raw.iter().map(|s| s.to_string()).collect::<Vec<_>>())
            .expect("parse command"),
    }
}

#[test]
fn command_run_routes_current_state_and_live_loop_variants() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("command-dispatch-state-loop");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    );

    let current_state_code = crate::command_run::run_with_exit_code(args(
        root.clone(),
        &[
            "current-state",
            "--receipt",
            "validation_artifacts/current-state-dispatch.json",
        ],
    ))
    .expect("current state dispatch");
    assert_eq!(current_state_code, 1);
    assert!(
        root.join("validation_artifacts/current-state-dispatch.json")
            .is_file()
    );

    let loop_code = crate::command_run::run_with_exit_code(args(
        root.clone(),
        &[
            "loop",
            "run",
            "--tier",
            "hot",
            "--cache-mode",
            "verified-local",
            "--jobs",
            "2",
            "--receipt",
            "validation_artifacts/observability/loop-dispatch.json",
        ],
    ))
    .expect("loop dispatch");
    assert_eq!(loop_code, 1);
    assert!(
        root.join("validation_artifacts/observability/loop-dispatch.json")
            .is_file()
    );
    std::fs::remove_dir_all(root).expect("cleanup state loop dispatch");
}

#[test]
fn nested_help_requests_do_not_execute_product_commands() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("command-dispatch-help");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    );

    for raw in [
        ["loop", "run", "--help"].as_slice(),
        ["loop", "run", "--tier", "hot", "--help"].as_slice(),
        ["loop", "measure", "--all", "--help"].as_slice(),
        ["loop", "format", "check", "--changed-rust", "--help"].as_slice(),
        ["loop", "format", "check", "--help", "--changed-rust"].as_slice(),
        ["namespace", "check", "--strict", "--help"].as_slice(),
        ["observe", "fit", "--help"].as_slice(),
        ["observe", "command-roundtrip", "--help"].as_slice(),
        ["loop", "help"].as_slice(),
    ] {
        let command =
            crate::parse_command(&raw.iter().map(|item| item.to_string()).collect::<Vec<_>>())
                .expect("help parse");
        assert!(matches!(command, crate::Command::Help));
        let code = crate::command_run::run_with_exit_code(crate::Args {
            root: root.clone(),
            command,
        })
        .expect("help dispatch");
        assert_eq!(code, 0);
    }

    assert!(
        !root
            .join("validation_artifacts/observability/loop-run.json")
            .exists(),
        "help must not mint live-loop receipts"
    );
    assert!(
        !root
            .join("validation_artifacts/observability/command-roundtrip")
            .exists(),
        "help must not mint command-roundtrip receipts"
    );
    std::fs::remove_dir_all(root).expect("cleanup command dispatch help");
}

#[test]
fn option_values_named_help_still_route_to_product_commands() {
    let command = crate::parse_command(
        &["observe", "metrics", "query", "--query", "help"]
            .into_iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
    )
    .expect("observe query parses");
    assert!(
        !matches!(command, crate::Command::Help),
        "observe query value named help must not become top-level help"
    );

    let archive = crate::parse_command(
        &[
            "archive",
            "build",
            "--receipt",
            "validation_artifacts/tmp/archive.json",
            "--zip",
            "help",
        ]
        .into_iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>(),
    )
    .expect("archive build parses");
    assert!(
        !matches!(archive, crate::Command::Help),
        "archive --zip value named help must not become top-level help"
    );
}
