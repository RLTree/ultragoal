use serde_json::{Value, json};
use std::path::{Path, PathBuf};

fn args(root: PathBuf, raw: &[&str]) -> crate::Args {
    crate::Args {
        root,
        command: crate::parse_command(&raw.iter().map(|s| s.to_string()).collect::<Vec<_>>())
            .expect("parse session command"),
    }
}

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn write_session_root(label: &str) -> PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    std::fs::create_dir_all(root.join(".codex-plugin")).expect("plugin dir");
    std::fs::create_dir_all(root.join("schemas")).expect("schemas dir");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"version":"0.0.11","resources":[]}),
    );
    write_json(
        &root.join(".codex-plugin/plugin.json"),
        &json!({"version":"0.0.11"}),
    );
    std::fs::copy(
        crate::self_tests::boundaries::workspace_fixtures::repo_root()
            .join("schemas/schema-authority-primitives.schema.json"),
        root.join("schemas/schema-authority-primitives.schema.json"),
    )
    .expect("common schema");
    std::fs::copy(
        crate::self_tests::boundaries::workspace_fixtures::repo_root()
            .join("schemas/session-log-hardening-receipt.schema.json"),
        root.join("schemas/session-log-hardening-receipt.schema.json"),
    )
    .expect("session schema");
    write_json(
        &root.join("schemas/schema-catalog.json"),
        &json!({
            "schema":"harness-ultragoal.schema-catalog.v1",
            "schemas":[
                {
                    "id":"https://harness-ultragoal.local/schemas/schema-authority-primitives.schema.json",
                    "path":"schemas/schema-authority-primitives.schema.json"
                },
                {
                    "id":"https://harness-ultragoal.local/schemas/session-log-hardening-receipt.schema.json",
                    "path":"schemas/session-log-hardening-receipt.schema.json"
                }
            ]
        }),
    );
    write_json(
        &root.join("validation_artifacts/harness/session-packet.json"),
        &json!({"status":"source-local-session-hardening"}),
    );
    let receipt = crate::self_tests::session::hardening::complete_receipt_for_root(&root);
    write_json(
        &root.join("validation_artifacts/harness/session-log-hardening-receipt.json"),
        &receipt,
    );
    root
}

#[test]
fn session_log_hardening_rebind_runs_through_cli_and_rejects_bad_surfaces() {
    let root = write_session_root("cli-session-rebind");
    let code = crate::command_run::run_with_exit_code(args(
        root.clone(),
        &[
            "session-log",
            "hardening",
            "rebind",
            "--receipt",
            "validation_artifacts/harness/session-log-hardening-receipt.json",
        ],
    ))
    .expect("session rebind command");
    assert_eq!(code, 0);
    let rebound = crate::json_boundary::read_json(
        &root.join("validation_artifacts/harness/session-log-hardening-receipt.json"),
    )
    .expect("rebound receipt");
    assert_eq!(rebound["status"], "pass");
    assert_eq!(rebound["candidate_version"], "0.0.11");
    assert_eq!(
        rebound["package_digest"],
        crate::package::inventory::package_digest(&root).expect("package digest")
    );

    let err = crate::cli::session::rebind(&root, Path::new("wrong-receipt.json"))
        .expect_err("wrong receipt path");
    assert!(err.contains("session-log hardening receipt must be"));

    std::fs::remove_dir_all(root).expect("cleanup session rebind");
}

#[test]
fn session_log_hardening_rebind_rejects_version_schema_and_path_edges() {
    let root = write_session_root("cli-session-bad-surfaces");
    write_json(
        &root.join(".codex-plugin/plugin.json"),
        &json!({"version":"0.0.12"}),
    );
    let err = crate::cli::session::rebind(
        &root,
        Path::new("validation_artifacts/harness/session-log-hardening-receipt.json"),
    )
    .expect_err("version mismatch");
    assert!(err.contains("manifest version 0.0.11 != plugin version 0.0.12"));
    write_json(&root.join(".codex-plugin/plugin.json"), &json!({}));
    let err = crate::cli::session::rebind(
        &root,
        Path::new("validation_artifacts/harness/session-log-hardening-receipt.json"),
    )
    .expect_err("missing plugin version");
    assert!(err.contains("missing manifest or plugin version"));
    write_json(
        &root.join(".codex-plugin/plugin.json"),
        &json!({"version":"0.0.11"}),
    );
    write_json(
        &root.join("validation_artifacts/harness/session-log-hardening-receipt.json"),
        &json!({"schema":"bad"}),
    );
    let err = crate::cli::session::rebind(
        &root,
        Path::new("validation_artifacts/harness/session-log-hardening-receipt.json"),
    )
    .expect_err("schema mismatch");
    assert!(err.contains("session-log hardening receipt did not validate"));
    std::fs::remove_dir_all(root).expect("cleanup bad surfaces");

    let symlink_root = write_session_root("cli-session-symlink-surface");
    std::fs::remove_dir_all(symlink_root.join("validation_artifacts"))
        .expect("remove validation dir");
    let target = symlink_root.join("outside-validation");
    std::fs::create_dir_all(&target).expect("outside target");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&target, symlink_root.join("validation_artifacts"))
        .expect("validation symlink");
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&target, symlink_root.join("validation_artifacts"))
        .expect("validation symlink");
    let err = crate::cli::session::rebind(
        &symlink_root,
        Path::new("validation_artifacts/harness/session-log-hardening-receipt.json"),
    )
    .expect_err("symlink path rejected");
    assert!(err.contains("session-log hardening receipt path escapes package root"));
    std::fs::remove_dir_all(symlink_root).expect("cleanup symlink root");
}
