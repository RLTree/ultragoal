use super::*;
use serde_json::json;
use std::fs;

mod edges;

#[test]
fn source_obligations_parse_requires_strict_and_supports_jobs() {
    assert!(
        parse(&["source-obligations".into(), "check".into()])
            .expect_err("strict required")
            .contains("--strict")
    );
    let command = parse(&[
        "source-obligations".into(),
        "check".into(),
        "--strict".into(),
        "--obligation".into(),
        "agent-queryable-observability".into(),
        "--jobs".into(),
        "2".into(),
    ])
    .expect("parse")
    .expect("command");
    assert_eq!(
        command.obligation.as_deref(),
        Some("agent-queryable-observability")
    );
    assert_eq!(command.jobs, Some(2));
    assert_eq!(command.receipt, PathBuf::from(RECEIPT_REL));
}

#[test]
fn source_obligations_command_writes_pass_and_fail_observability() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("source-obligations-command");
    write_minimal_root(&root, true);
    let command = SourceObligationsCommand {
        obligation: Some("agent-queryable-observability".to_string()),
        receipt: PathBuf::from(RECEIPT_REL),
        jobs: None,
    };
    assert_eq!(run(&root, &command).expect("pass run"), 0);
    let pass = crate::json_boundary::read_json(&root.join(RECEIPT_REL)).expect("pass receipt");
    assert_eq!(pass["status"], "pass");
    assert_eq!(pass["operation"], "source-obligations.check");
    assert_eq!(pass["event"]["command"], "ultragoal source-obligations");
    assert_eq!(pass["event"]["subcommand"], "check");
    assert_eq!(pass["event"]["task_count"], 1);
    assert_eq!(pass["event"]["cache_mode"], "declared_local");
    assert_eq!(stdout::contract(&pass).len(), 1);

    write_minimal_root(&root, false);
    assert_eq!(run(&root, &command).expect("fail run"), 1);
    let fail = crate::json_boundary::read_json(&root.join(RECEIPT_REL)).expect("fail receipt");
    assert_eq!(fail["status"], "fail");
    assert_eq!(
        fail["event"]["failure_class"],
        "source_obligations_check_failure"
    );
    assert!(fail["supported_claims"].as_array().unwrap().is_empty());
    assert!(
        stdout::contract(&fail).iter().any(
            |line| line.contains("failed_check=source-obligations-check-observability-binding")
        )
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn default_source_obligations_check_uses_parallel_scheduler() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("source-obligations-parallel");
    write_parallel_fixture(&root);
    let scheduler = SchedulerConfig::from_jobs(Some(4)).expect("scheduler");
    let result = validate(&root, None, scheduler);
    let metrics = result.scheduler_metrics.first().expect("metrics");
    assert!(metrics.task_count > 1);
    assert!(metrics.worker_count > 1 || metrics.task_count == 1);
    assert_eq!(metrics.task_class, TaskClass::PureReadParallel.id());
    assert!(metrics.deterministic_ordering);
    assert!(!metrics.shared_validation_artifact_writes_allowed);
    fs::remove_dir_all(root).expect("cleanup");
}

fn write_minimal_root(root: &Path, valid: bool) {
    fs::create_dir_all(root.join("docs")).expect("docs");
    crate::json_boundary::write_json(&root.join("plugin-manifest-draft.json"), &json!({}))
        .expect("manifest");
    let disposition = if valid { "mechanized" } else { "" };
    crate::json_boundary::write_json(
        &root.join(MATRIX_REL),
        &json!({
            "obligations": [{
                "id": "agent-queryable-observability",
                "obligation": "observability must be queryable by agents",
                "package_surface": "cli",
                "enforcement_disposition": disposition,
                "coverage_status": "validator red fixture receipt",
                "missing_validation_fixture_or_receipt": "none",
                "claim_ceiling_impact": "blocks readiness release completion update_goal"
            }]
        }),
    )
    .expect("matrix");
}

fn write_parallel_fixture(root: &Path) {
    fs::create_dir_all(root.join("docs")).expect("docs");
    crate::json_boundary::write_json(&root.join("plugin-manifest-draft.json"), &json!({}))
        .expect("manifest");
    crate::json_boundary::write_json(
        &root.join(MATRIX_REL),
        &json!({
            "obligations": [
                row("agent-queryable-observability"),
                row("source-obligation-parity-anti-bundling"),
                row("scheduler-runner-tracker-boundaries")
            ]
        }),
    )
    .expect("matrix");
}

fn row(id: &str) -> serde_json::Value {
    json!({
        "id": id,
        "obligation": format!("{id} obligation"),
        "package_surface": "cli",
        "enforcement_disposition": "mechanized validator red fixture receipt",
        "coverage_status": "validator red fixture receipt",
        "missing_validation_fixture_or_receipt": "none",
        "claim_ceiling_impact": "blocks readiness release completion update_goal"
    })
}
