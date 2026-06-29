use serde_json::json;
use std::path::{Path, PathBuf};

fn write_json(path: &Path, value: &serde_json::Value) {
    if let Some(parent) = path.parent() {
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
fn command_run_routes_audit_and_performance_variants() {
    let root = crate::self_tests::boundaries::support::temp_root("command-dispatch-routes");
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
    std::fs::remove_dir_all(root).expect("cleanup command dispatch routes");
}
