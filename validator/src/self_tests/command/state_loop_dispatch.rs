use serde_json::json;
use std::path::{Path, PathBuf};

fn write_json(path: &Path, value: &serde_json::Value) {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).expect("parent");
    }
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
    let root = crate::self_tests::boundaries::support::temp_root("command-dispatch-state-loop");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    );

    assert!(matches!(
        crate::parse_command(&["current-state".to_string(), "--json".to_string()])
            .expect("current-state parse"),
        crate::Command::CurrentState(_)
    ));
    assert!(matches!(
        crate::parse_command(
            &["loop", "run", "--tier", "hot"]
                .into_iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        )
        .expect("loop parse"),
        crate::Command::LiveLoop(_)
    ));

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
