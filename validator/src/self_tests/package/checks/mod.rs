use serde_json::{Value, json};
use std::path::Path;

mod scheduler;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

#[test]
fn package_checks_route_schema_inventory_and_skill_failures() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("package-checks");
    std::fs::create_dir_all(root.join("fixtures/valid")).expect("fixtures");
    std::fs::create_dir_all(root.join("skills/demo")).expect("skill dir");
    std::fs::create_dir_all(root.join("__pycache__")).expect("bytecode dir");
    std::fs::create_dir_all(root.join("artifacts/root-receipts")).expect("artifacts");
    std::fs::write(root.join("fixtures/valid/bad.json"), "{}").expect("bad fixture");
    std::fs::write(root.join("root-ref.md"), "root").expect("root ref");
    std::fs::write(root.join("skills/demo/SKILL.md"), "`root-ref.md`\n").expect("skill");
    std::fs::write(root.join("__pycache__/x.pyc"), "bytecode").expect("bytecode");
    std::fs::write(root.join("artifacts/root-receipts/post-merge.json"), "{}")
        .expect("root verification stage artifact");
    std::fs::write(
        root.join("README.md"),
        "Current digest sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n",
    )
    .expect("moving value");
    std::fs::write(
        root.join("REPORT.md"),
        "gpt-5.4-mini provisional approval\n",
    )
    .expect("stale review");
    let private_home = ["/", "Users", "/", "example", "/", "private-proof\n"].concat();
    std::fs::write(root.join("private.txt"), private_home).expect("private");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({
            "skills":[{"name":"demo","path":"skills/demo/SKILL.md"}],
            "agents":[],
            "schemas":[],
            "fixtures":["fixtures/valid/bad.json","fixtures/valid/bad.json","missing.json"],
            "authorable_templates":[],
            "generated_examples":[],
            "resources":[
                "../escape.txt",
                "artifacts/root-receipts/post-merge.json",
                "private.txt"
            ]
        }),
    );
    let store = crate::schema_catalog::load(
        &crate::self_tests::boundaries::workspace_fixtures::repo_root(),
    );
    let check_ids = [
        "schema-valid",
        "plugin-inventory-exactly-once",
        "plugin-inventory-closure",
        "skill-inventory-closure",
        "namespace-progressive-disclosure",
        "source-obligation-coverage",
        "agent-standards-enforcement",
        "validator-execution-provenance",
        "cli-control-plane-authority",
        "cli-self-law-compliance",
        "cli-performance-latency-speed-iteration-fitness",
    ]
    .into_iter()
    .map(str::to_string)
    .collect::<Vec<_>>();
    let failures = crate::audit::package::checks::checks_with_scheduler(
        &root,
        &store,
        &check_ids,
        crate::scheduler::SchedulerConfig::from_jobs(None).expect("default scheduler"),
    )
    .failures;

    let schema = failures.get("schema-valid").cloned().unwrap_or_default();
    assert!(
        schema
            .iter()
            .any(|item| item.contains("fixtures/valid/bad.json"))
    );
    let duplicate = failures
        .get("plugin-inventory-exactly-once")
        .cloned()
        .unwrap_or_default();
    assert!(duplicate.iter().any(|item| item.contains("duplicates=")));
    let closure = failures
        .get("plugin-inventory-closure")
        .cloned()
        .unwrap_or_default();
    assert!(closure.iter().any(|item| item.contains("missing.json")));
    assert!(
        closure
            .iter()
            .any(|item| item.contains("root_verification_receipt_resource_packaged"))
    );
    assert!(
        closure
            .iter()
            .any(|item| item.contains("manifest_owned_private_local_path"))
    );
    let provenance = failures
        .get("validator-execution-provenance")
        .cloned()
        .unwrap_or_default();
    assert!(
        provenance
            .iter()
            .any(|item| item.contains("moving_value_drift_in_stable_text"))
    );
    assert!(
        provenance
            .iter()
            .any(|item| item.contains("stale_review_cadence_in_current_surface"))
    );
    let manifest = crate::json_boundary::read_json(&root.join("plugin-manifest-draft.json"))
        .expect("manifest");
    let skill = crate::skill_links::manifest_failures(&root, &manifest);
    assert!(skill.iter().any(|item| item.detail.contains("root-ref.md")));
    std::fs::remove_dir_all(root).expect("cleanup package checks");
}

#[test]
fn package_checks_report_missing_manifest_as_inventory_failure() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("package-checks-no-manifest");
    std::fs::create_dir_all(&root).expect("root");
    let store = crate::schema_catalog::load(
        &crate::self_tests::boundaries::workspace_fixtures::repo_root(),
    );
    let failures = crate::audit::package::checks::checks_with_scheduler(
        &root,
        &store,
        &[],
        crate::scheduler::SchedulerConfig::from_jobs(None).expect("default scheduler"),
    )
    .failures;
    assert!(
        failures
            .get("plugin-inventory-closure")
            .is_some_and(|items| items.iter().any(|item| item.contains("load failed")))
    );
    std::fs::remove_dir_all(root).expect("cleanup no manifest");
}
