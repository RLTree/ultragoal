use std::path::PathBuf;

fn args(root: PathBuf, raw: &[&str]) -> crate::Args {
    crate::Args {
        root,
        command: crate::parse_command(&raw.iter().map(|s| s.to_string()).collect::<Vec<_>>())
            .expect("parse command"),
    }
}

#[test]
fn command_run_routes_openai_authority_proof_command() {
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
}
