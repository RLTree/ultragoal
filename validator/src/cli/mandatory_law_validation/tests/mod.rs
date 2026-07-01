use super::*;
use serde_json::{Value, json};
use std::fs;

mod edges;

#[test]
fn mandatory_law_validation_parse_supports_default_targeted_and_alias() {
    let default = parse(&["mandatory-law".into(), "validation".into()])
        .expect("parse")
        .expect("command");
    assert_eq!(default.receipt, PathBuf::from(RECEIPT_REL));
    assert!(default.law.is_none());

    let targeted = parse(&[
        "mandatory-law-validation".into(),
        "--law".into(),
        "schema-valid".into(),
        "--jobs".into(),
        "2".into(),
    ])
    .expect("targeted parse")
    .expect("targeted command");
    assert_eq!(targeted.law.as_deref(), Some("schema-valid"));
    assert_eq!(targeted.jobs, Some(2));

    assert!(parse(&["source".into(), "audit".into()]).unwrap().is_none());
}

#[test]
fn mandatory_law_validation_command_writes_pass_and_fail_observability() {
    let root = crate::self_tests::boundaries::support::temp_root("mandatory-law-command");
    create_law_package(&root);
    let command = MandatoryLawValidationCommand {
        law: Some("schema-valid".to_string()),
        receipt: PathBuf::from(RECEIPT_REL),
        jobs: None,
    };
    assert_eq!(run(&root, &command).expect("pass run"), 0);
    let pass = crate::json_boundary::read_json(&root.join(RECEIPT_REL)).expect("pass receipt");
    assert_eq!(pass["status"], "pass");
    assert_eq!(pass["operation"], "mandatory-law.validation");
    assert_eq!(pass["event"]["command"], "ultragoal mandatory-law");
    assert_eq!(pass["event"]["task_count"], 1);
    assert_eq!(pass["event"]["cache_mode"], "declared_local");
    assert_eq!(stdout::contract(&pass).len(), 1);

    crate::json_boundary::write_json(&root.join("docs/evidence.json"), &json!({"changed":true}))
        .expect("stale evidence");
    assert_eq!(run(&root, &command).expect("fail run"), 1);
    let fail = crate::json_boundary::read_json(&root.join(RECEIPT_REL)).expect("fail receipt");
    assert_eq!(fail["status"], "fail");
    assert_eq!(
        fail["event"]["failure_class"],
        "mandatory_law_validation_failure"
    );
    assert!(fail["supported_claims"].as_array().unwrap().is_empty());
    assert!(
        stdout::contract(&fail).iter().any(
            |line| line.contains("failed_check=mandatory-law-validation-observability-binding")
        )
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn default_mandatory_law_validation_uses_parallel_scheduler() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let scheduler = SchedulerConfig::from_jobs(Some(4)).expect("scheduler");
    let result = validate(&root, None, scheduler);
    let metrics = result.scheduler_metrics.first().expect("metrics");
    assert!(metrics.task_count > 1);
    assert!(metrics.worker_count > 1 || metrics.task_count == 1);
    assert_eq!(metrics.task_class, TaskClass::PureReadParallel.id());
    assert!(metrics.deterministic_ordering);
    assert!(!metrics.shared_validation_artifact_writes_allowed);
}

fn create_law_package(root: &Path) {
    for dir in [
        "docs",
        "fixtures/red",
        "fixtures/mandatory-law-surfaces/valid",
    ] {
        fs::create_dir_all(root.join(dir)).expect("dir");
    }
    crate::json_boundary::write_json(&root.join("plugin-manifest-draft.json"), &json!({}))
        .expect("manifest");
    crate::json_boundary::write_json(&root.join("docs/evidence.json"), &json!({"ok":true}))
        .expect("evidence");
    let evidence_digest = crate::digest::file(&root.join("docs/evidence.json")).expect("digest");
    let row = law_row(&evidence_digest);
    crate::json_boundary::write_json(
        &root.join("docs/mandatory-law-surfaces.json"),
        &json!({
            "schema": "harness-ultragoal.mandatory-law-surfaces.v1",
            "laws": [row.clone()]
        }),
    )
    .expect("registry");
    crate::json_boundary::write_json(
        &root.join("fixtures/mandatory-law-surfaces/valid/schema-valid.json"),
        &row,
    )
    .expect("valid fixture");
    crate::json_boundary::write_json(
        &root.join("fixtures/red/schema-valid-schema-dispatch-red.json"),
        &json!({
            "id": "schema-valid-schema-dispatch-red",
            "expected_failure": {
                "error": "mandatory_law_specific_guard_not_enforced:schema-valid:schema_dispatch"
            }
        }),
    )
    .expect("red fixture");
}

fn law_row(evidence_digest: &str) -> Value {
    json!({
        "schema": "harness-ultragoal.mandatory-law-surface-receipt.v1",
        "law_id": "schema-valid",
        "enforcement_status": "deterministic_fail_closed",
        "standards_row_id": "schema-valid",
        "source_obligation_id": "schema-valid",
        "foundational_trace_id": "schema-valid",
        "validator_check_id": "schema-valid",
        "valid_fixture_path": "fixtures/mandatory-law-surfaces/valid/schema-valid.json",
        "claim_ceiling_guard": "schema_claims_withheld_without_schema_validation",
        "red_fixture_ids": ["schema-valid-schema-dispatch-red"],
        "behavior_failure_modes": ["schema_dispatch"],
        "law_specific": {"schema_dispatch": true},
        "evidence_artifacts": [{"path": "docs/evidence.json", "digest": evidence_digest}]
    })
}
