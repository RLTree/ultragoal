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
fn command_run_routes_audit_variant() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("command-dispatch-routes");
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
    let err = crate::command_run::run_with_exit_code(args(
        root.clone(),
        &[
            "source",
            "audit",
            "--receipt",
            "receipt.json",
            "--red-report",
            "not-red-report.json",
        ],
    ))
    .expect_err("bad red-report basename rejected");
    assert!(err.contains("red-fixture-report.json"), "{err}");

    std::fs::remove_dir_all(root).expect("cleanup command dispatch routes");
}
