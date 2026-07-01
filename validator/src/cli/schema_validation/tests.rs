use super::*;
use serde_json::{Value, json};
use std::fs;

#[test]
fn schema_validation_parse_supports_default_targeted_and_alias() {
    let default = parse(&["schema".into(), "validation".into()])
        .expect("parse")
        .expect("command");
    assert_eq!(default.receipt, PathBuf::from(RECEIPT_REL));
    assert!(default.schema.is_none());

    let targeted = parse(&[
        "schema-validation".into(),
        "--schema".into(),
        "test.schema.json".into(),
        "--file".into(),
        "sample.json".into(),
        "--jobs".into(),
        "2".into(),
    ])
    .expect("targeted parse")
    .expect("targeted command");
    assert_eq!(targeted.schema.as_deref(), Some("test.schema.json"));
    assert_eq!(targeted.file, Some(PathBuf::from("sample.json")));
    assert_eq!(targeted.jobs, Some(2));

    assert!(parse(&["source".into(), "audit".into()]).unwrap().is_none());
    assert!(
        parse(&[
            "schema".into(),
            "validation".into(),
            "--schema".into(),
            "x".into()
        ])
        .expect_err("schema alone rejected")
        .contains("--schema and --file")
    );
}

#[test]
fn schema_validation_command_writes_pass_and_fail_observability() {
    let root = crate::self_tests::boundaries::support::temp_root("schema-validation-command");
    create_schema_package(&root, json!({"name": "Tree"}));
    let command = SchemaValidationCommand {
        schema: Some("test.schema.json".to_string()),
        file: Some(PathBuf::from("sample.json")),
        receipt: PathBuf::from(RECEIPT_REL),
        jobs: None,
    };
    assert_eq!(run(&root, &command).expect("pass run"), 0);
    let pass = crate::json_boundary::read_json(&root.join(RECEIPT_REL)).expect("pass receipt");
    assert_eq!(pass["status"], "pass");
    assert_eq!(pass["operation"], "schema.validation");
    assert_eq!(pass["event"]["command"], "ultragoal schema");
    assert_eq!(pass["event"]["subcommand"], "validation");
    assert_eq!(pass["event"]["task_count"], 1);
    assert_eq!(pass["event"]["cache_mode"], "declared_local");
    assert_eq!(stdout::contract(&pass).len(), 1);

    crate::json_boundary::write_json(&root.join("sample.json"), &json!({})).expect("sample");
    assert_eq!(run(&root, &command).expect("fail run"), 1);
    let fail = crate::json_boundary::read_json(&root.join(RECEIPT_REL)).expect("fail receipt");
    assert_eq!(fail["status"], "fail");
    assert_eq!(fail["event"]["failure_class"], "schema_validation_failure");
    assert!(fail["supported_claims"].as_array().unwrap().is_empty());
    assert!(
        stdout::contract(&fail)
            .iter()
            .any(|line| line.contains("failed_check=schema-validation-observability-binding"))
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn default_schema_validation_uses_mapped_parallel_path() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let store = crate::schema_catalog::load(&root);
    let scheduler = SchedulerConfig::from_jobs(Some(4)).expect("scheduler");
    let result = mapped(&root, &store, scheduler);
    let metrics = result.scheduler_metrics.first().expect("metrics");
    assert!(metrics.task_count > 1);
    assert!(metrics.worker_count > 1 || metrics.task_count == 1);
    assert_eq!(metrics.task_class, TaskClass::PureReadParallel.id());
    assert!(metrics.deterministic_ordering);
    assert!(!metrics.shared_validation_artifact_writes_allowed);
}

fn create_schema_package(root: &Path, sample: Value) {
    fs::create_dir_all(root.join("schemas")).expect("schemas");
    crate::json_boundary::write_json(&root.join("plugin-manifest-draft.json"), &json!({}))
        .expect("manifest");
    crate::json_boundary::write_json(
        &root.join("schemas/schema-catalog.json"),
        &json!({"schemas":[{"id":"test.schema.json","path":"schemas/test.schema.json"}]}),
    )
    .expect("catalog");
    crate::json_boundary::write_json(
        &root.join("schemas/test.schema.json"),
        &json!({
            "$id": "test.schema.json",
            "type": "object",
            "required": ["name"],
            "properties": {"name": {"type": "string"}},
            "additionalProperties": false
        }),
    )
    .expect("schema");
    crate::json_boundary::write_json(&root.join("sample.json"), &sample).expect("sample");
}
