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
fn command_run_routes_external_authority_proof_commands() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("authority-dispatch-routes");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    write_json(&root.join("schemas/schema-catalog.json"), &json!([]));
    write_json(
        &root.join("docs/mandatory-law-surfaces.json"),
        &json!({"laws":[]}),
    );
    write_json(
        &root.join("docs/source-obligation-matrix.json"),
        &json!({"obligations":[]}),
    );
    write_json(
        &root.join("templates/agent-standards/enforcement.json"),
        &json!({"rows":[]}),
    );
    write_json(&root.join("templates/RED_FIXTURES.json"), &json!([]));

    for (label, raw, receipt) in [
        (
            "halo",
            vec![
                "halo",
                "capability",
                "prove",
                "--receipt",
                "validation_artifacts/halo/halo.json",
                "--app-path",
                "Missing.app",
            ],
            "validation_artifacts/halo/halo.json",
        ),
        (
            "improvement-loop",
            vec![
                "improvement-loop",
                "prove",
                "--receipt",
                "validation_artifacts/improvement-loop/improvement.json",
            ],
            "validation_artifacts/improvement-loop/improvement.json",
        ),
        (
            "promptfoo",
            vec![
                "promptfoo",
                "prove",
                "--receipt",
                "validation_artifacts/promptfoo/promptfoo.json",
                "--promptfoo-bin",
                "missing-promptfoo",
            ],
            "validation_artifacts/promptfoo/promptfoo.json",
        ),
    ] {
        let result = crate::command_run::run_with_exit_code(args(root.clone(), &raw));
        assert!(result.is_ok(), "{label} dispatch failed: {result:?}");
        let code = result.expect("checked dispatch result");
        assert_eq!(code, 1, "{label} should fail closed");
        assert!(root.join(receipt).is_file(), "{label} receipt missing");
    }

    let openai_root = crate::self_tests::openai::prepare_root(
        "command-dispatch-openai",
        &[
            ".gitignore",
            "docs/openai-key-policy.json",
            "docs/openai-provider-policy.json",
        ],
    );
    let openai_code = crate::command_run::run_with_exit_code(args(
        openai_root.clone(),
        &[
            "openai",
            "config",
            "prove",
            "--receipt",
            "validation_artifacts/openai/dispatch-config.json",
        ],
    ))
    .expect("openai dispatch");
    assert_eq!(openai_code, 0);
    assert!(
        openai_root
            .join("validation_artifacts/openai/dispatch-config.json")
            .is_file()
    );
    std::fs::remove_dir_all(openai_root).expect("cleanup openai dispatch");
    std::fs::remove_dir_all(root).expect("cleanup authority dispatch routes");
}
