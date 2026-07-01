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
fn source_audit_parser_accepts_bounded_jobs_and_rejects_non_numeric_jobs() {
    let raw = [
        "source",
        "audit",
        "--receipt",
        "receipt.json",
        "--jobs",
        "4",
    ]
    .iter()
    .map(|item| item.to_string())
    .collect::<Vec<_>>();
    let parsed = crate::parse_command(&raw).expect("parse jobs");
    assert!(matches!(
        parsed,
        crate::Command::Audit { jobs: Some(4), .. }
    ));

    let bad = [
        "source",
        "audit",
        "--receipt",
        "receipt.json",
        "--jobs",
        "many",
    ]
    .iter()
    .map(|item| item.to_string())
    .collect::<Vec<_>>();
    let err = crate::parse_command(&bad).expect_err("non-numeric jobs rejected");
    assert!(err.contains("invalid numeric value for --jobs"), "{err}");

    let bad_mode = [
        "source",
        "audit",
        "--receipt",
        "receipt.json",
        "--mode",
        "slow",
    ]
    .iter()
    .map(|item| item.to_string())
    .collect::<Vec<_>>();
    let err = crate::parse_command(&bad_mode).expect_err("unknown audit mode rejected");
    assert!(err.contains("invalid source audit --mode: slow"), "{err}");

    let target = [
        "target-repo",
        "audit",
        "--surface-root",
        "target",
        "--receipt",
        "receipt.json",
        "--mode",
        "focused",
        "--require-observability",
        "--require-product-cohesion",
        "--jobs",
        "3",
    ]
    .iter()
    .map(|item| item.to_string())
    .collect::<Vec<_>>();
    let parsed = crate::parse_command(&target).expect("target audit parse");
    assert!(matches!(
        parsed,
        crate::Command::Audit {
            jobs: Some(3),
            require_observability: true,
            require_product_cohesion: true,
            ..
        }
    ));

    let bad_target_mode = [
        "target-repo",
        "audit",
        "--surface-root",
        "target",
        "--receipt",
        "receipt.json",
        "--mode",
        "slow",
    ]
    .iter()
    .map(|item| item.to_string())
    .collect::<Vec<_>>();
    let err = crate::parse_command(&bad_target_mode).expect_err("target mode rejected");
    assert!(
        err.contains("invalid target-repo audit --mode: slow"),
        "{err}"
    );

    assert!(matches!(
        crate::parse_command(&["package".to_string(), "digest".to_string()]).expect("package"),
        crate::Command::PackageDigest
    ));
}

#[test]
fn command_run_routes_audit_and_performance_variants() {
    let root = crate::self_tests::boundaries::support::temp_root("command-dispatch-routes");
    let empty_path_write = std::panic::catch_unwind(|| write_json(Path::new(""), &json!({})));
    assert!(empty_path_write.is_err());
    let leaf = std::path::Path::new("command-dispatch-leaf.json");
    write_json(leaf, &json!({"leaf": true}));
    std::fs::remove_file(leaf).expect("cleanup leaf");
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
    let receipt = root.join("receipt.json");
    let err = crate::command_run::run_with_exit_code(args(
        root.clone(),
        &[
            "source",
            "audit",
            "--receipt",
            receipt.to_str().expect("receipt"),
            "--red-report",
            "not-red-report.json",
        ],
    ))
    .expect_err("bad red-report basename rejected");
    assert!(err.contains("red-fixture-report.json"), "{err}");

    let performance_receipt = root.join("performance.json");
    let code = crate::command_run::run_with_exit_code(args(
        root.clone(),
        &[
            "performance",
            "budgets",
            "--class",
            "focused",
            "--receipt",
            performance_receipt.to_str().expect("performance receipt"),
        ],
    ))
    .expect("performance command");
    assert_eq!(code, 0);
    assert!(performance_receipt.is_file());
    let performance = crate::json_boundary::read_json(&performance_receipt).expect("performance");
    assert_eq!(performance["budget"]["class"], "focused_repair");
    assert_eq!(performance["budget"]["target_ms"], 15_000);

    for (label, raw, receipt) in [
        (
            "halo",
            vec![
                "halo",
                "capability",
                "prove",
                "--receipt",
                "halo.json",
                "--app-path",
                "Missing.app",
            ],
            "halo.json",
        ),
        (
            "improvement-loop",
            vec!["improvement-loop", "prove", "--receipt", "improvement.json"],
            "improvement.json",
        ),
        (
            "promptfoo",
            vec![
                "promptfoo",
                "prove",
                "--receipt",
                "promptfoo.json",
                "--promptfoo-bin",
                "missing-promptfoo",
            ],
            "promptfoo.json",
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
    std::fs::remove_dir_all(root).expect("cleanup command dispatch routes");
}
