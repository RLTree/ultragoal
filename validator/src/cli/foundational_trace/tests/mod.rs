use super::*;
use serde_json::json;
use std::fs;

mod edges;

#[test]
fn foundational_trace_parse_requires_strict_and_supports_jobs() {
    assert!(
        parse(&["foundational-trace".into(), "check".into()])
            .expect_err("strict required")
            .contains("--strict")
    );
    let command = parse(&[
        "foundational-trace".into(),
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
fn foundational_trace_command_writes_pass_and_fail_observability() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("foundational-trace-command");
    write_minimal_root(&root, &["agent-queryable-observability"], true);
    let command = FoundationalTraceCommand {
        obligation: Some("agent-queryable-observability".to_string()),
        receipt: PathBuf::from(RECEIPT_REL),
        jobs: None,
    };
    assert_eq!(run(&root, &command).expect("pass run"), 0);
    let pass = crate::json_boundary::read_json(&root.join(RECEIPT_REL)).expect("pass receipt");
    assert_eq!(pass["status"], "pass");
    assert_eq!(pass["operation"], "foundational-trace.check");
    assert_eq!(pass["event"]["command"], "ultragoal foundational-trace");
    assert_eq!(pass["event"]["subcommand"], "check");
    assert_eq!(pass["event"]["task_count"], 2);
    assert_eq!(pass["event"]["cache_mode"], "declared_local");
    assert_eq!(stdout::contract(&pass).len(), 1);

    write_minimal_root(&root, &["agent-queryable-observability"], false);
    assert_eq!(run(&root, &command).expect("fail run"), 1);
    let fail = crate::json_boundary::read_json(&root.join(RECEIPT_REL)).expect("fail receipt");
    assert_eq!(fail["status"], "fail");
    assert_eq!(
        fail["event"]["failure_class"],
        "foundational_trace_check_failure"
    );
    assert!(fail["supported_claims"].as_array().unwrap().is_empty());
    assert!(
        stdout::contract(&fail).iter().any(
            |line| line.contains("failed_check=foundational-trace-check-observability-binding")
        )
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn default_foundational_trace_check_uses_parallel_scheduler() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("foundational-trace-parallel");
    write_minimal_root(
        &root,
        &[
            "agent-queryable-observability",
            "source-obligation-parity-anti-bundling",
            "scheduler-runner-tracker-boundaries",
        ],
        true,
    );
    let scheduler = SchedulerConfig::from_jobs(Some(4)).expect("scheduler");
    let result = validate(&root, None, scheduler);
    assert!(result.failures.is_empty(), "{:?}", result.failures);
    let metrics = result.scheduler_metrics.first().expect("metrics");
    assert!(metrics.task_count > 1);
    assert!(metrics.worker_count > 1 || metrics.task_count == 1);
    assert_eq!(metrics.task_class, TaskClass::PureReadParallel.id());
    assert!(metrics.deterministic_ordering);
    assert!(!metrics.shared_validation_artifact_writes_allowed);
    fs::remove_dir_all(root).expect("cleanup");
}

fn write_minimal_root(root: &Path, ids: &[&str], valid: bool) {
    fs::create_dir_all(root.join("docs")).expect("docs");
    fs::create_dir_all(root.join("templates/agent-standards")).expect("standards");
    crate::json_boundary::write_json(&root.join("plugin-manifest-draft.json"), &json!({}))
        .expect("manifest");
    crate::json_boundary::write_json(
        &root.join(MATRIX_REL),
        &json!({
            "obligations": ids.iter().map(|id| json!({
                "id": id,
                "obligation": format!("{id} obligation"),
                "package_surface": "cli",
                "enforcement_disposition": "mechanized validator red fixture receipt",
                "coverage_status": "validator red fixture receipt",
                "missing_validation_fixture_or_receipt": "none",
                "claim_ceiling_impact": "blocks readiness release completion update_goal"
            })).collect::<Vec<_>>()
        }),
    )
    .expect("matrix");
    crate::json_boundary::write_json(
        &root.join("templates/agent-standards/enforcement.json"),
        &json!({
            "rows": [{
                "id": "agent-queryable-observability",
                "title": "Agent-queryable observability"
            }]
        }),
    )
    .expect("standards");
    crate::json_boundary::write_json(
        &root.join("templates/RED_FIXTURES.json"),
        &json!([{
            "id": "agent-queryable-observability-missing-log-event",
            "path": "fixtures/red/agent-queryable-observability-missing-log-event.json"
        }]),
    )
    .expect("red catalog");
    let digest = if valid {
        crate::digest::file(&root.join(MATRIX_REL)).expect("matrix digest")
    } else {
        crate::digest::ZERO.to_string()
    };
    crate::json_boundary::write_json(
        &root.join(TRACE_REL),
        &json!({
            "entries": ids.iter().map(|id| trace_row(id, &digest)).collect::<Vec<_>>()
        }),
    )
    .expect("trace");
}

fn trace_row(id: &str, digest: &str) -> serde_json::Value {
    json!({
        "obligation_id": id,
        "law_id": "full-local-observability-stack-integration-non-opaque-failure",
        "standards_row_id": "agent-queryable-observability",
        "validator_check_id": "agent-queryable-observability",
        "red_fixture_id": "agent-queryable-observability-missing-log-event",
        "valid_fixture_id": "fixtures/valid/agent-queryable-observability.json",
        "receipt_requirement": "source-local observability receipt current",
        "claim_ceiling_impact": "blocks readiness release completion update_goal",
        "source_artifact": {
            "path": MATRIX_REL,
            "digest": digest
        }
    })
}
