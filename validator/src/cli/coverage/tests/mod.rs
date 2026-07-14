use super::*;
use serde_json::json;
use std::fs;

mod execution_edges;
mod fixture;
mod validation;
use fixture::{coverage_receipt, write_coverage_root};

#[test]
fn coverage_prove_parse_supports_receipt_jobs_and_validate_existing() {
    let command = parse(&[
        "coverage".into(),
        "prove".into(),
        "--receipt".into(),
        "validation_artifacts/coverage/custom.json".into(),
        "--jobs".into(),
        "2".into(),
        "--validate-existing".into(),
    ])
    .expect("parse")
    .expect("command");
    assert_eq!(
        command.receipt,
        PathBuf::from("validation_artifacts/coverage/custom.json")
    );
    assert_eq!(command.jobs, Some(2));
    assert!(command.validate_existing);
    assert_eq!(command.mode, CoverageMode::Strict);
    let routine = parse(&[
        "coverage".into(),
        "prove".into(),
        "--mode".into(),
        "routine".into(),
    ])
    .expect("parse routine")
    .expect("routine command");
    assert_eq!(routine.mode, CoverageMode::Routine);
    assert!(parse(&["source".into(), "audit".into()]).unwrap().is_none());
    assert!(
        parse(&["coverage".into(), "prove".into(), "--bad".into()])
            .expect_err("unknown")
            .contains("unknown coverage prove argument")
    );
}

#[test]
fn coverage_prove_command_writes_pass_and_fail_observability() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("coverage-prove-command");
    write_coverage_root(&root, 100.0, json!([]));
    let command = CoverageCommand {
        receipt: PathBuf::from(COVERAGE_RECEIPT_REL),
        jobs: Some(4),
        validate_existing: true,
        mode: CoverageMode::Strict,
    };
    assert_eq!(run(&root, &command).expect("pass run"), 0);
    let pass =
        crate::json_boundary::read_json(&root.join(OBSERVABILITY_RECEIPT_REL)).expect("pass");
    assert_eq!(pass["status"], "pass");
    assert_eq!(pass["operation"], "coverage.prove");
    assert_eq!(pass["event"]["command"], "ultragoal coverage");
    assert_eq!(pass["event"]["worker_count"], 1);
    assert_eq!(pass["event"]["task_count"], 1);
    assert_eq!(pass["event"]["queue_depth"], 1);
    assert_eq!(
        pass["event"]["cache_mode"],
        "coverage_validate_existing_no_cache"
    );
    assert!(
        pass["event"]["saturation_status"]
            .as_str()
            .unwrap()
            .contains("shared_authority_write_serial")
    );
    assert_eq!(stdout::contract(&pass).len(), 1);

    write_coverage_root(
        &root,
        99.0,
        json!([{
            "path":"src/lib.rs",
            "reason":"line coverage 99.00%",
            "owner":"coverage-test-owner",
            "blocker_or_debt_id":"coverage-test-gap"
        }]),
    );
    assert_eq!(run(&root, &command).expect("fail run"), 1);
    let fail =
        crate::json_boundary::read_json(&root.join(OBSERVABILITY_RECEIPT_REL)).expect("fail");
    assert_eq!(fail["status"], "fail");
    assert_eq!(fail["event"]["failure_class"], "coverage_prove_failure");
    assert!(fail["supported_claims"].as_array().unwrap().is_empty());
    let fail_lines = stdout::contract(&fail);
    assert!(fail_lines[0].contains("trace_id=trace-"));
    assert!(fail_lines[0].contains("failure_class=coverage_prove_failure"));
    assert!(
        fail_lines
            .iter()
            .any(|line| line.contains("failed_check=coverage-prove-observability-binding"))
    );
    assert!(
        fail_lines[1].contains("failure_class=coverage_prove_failure")
            && fail_lines[1].contains("--correlation-id corr-")
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn coverage_prove_records_authoritative_command_failure() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("coverage-prove-executor");
    write_coverage_root(&root, 100.0, json!([]));
    let command = CoverageCommand {
        receipt: PathBuf::from(COVERAGE_RECEIPT_REL),
        jobs: None,
        validate_existing: false,
        mode: CoverageMode::Strict,
    };
    assert_eq!(
        run_with_executor(&root, &command, failing_executor).expect("run"),
        1
    );
    let receipt =
        crate::json_boundary::read_json(&root.join(OBSERVABILITY_RECEIPT_REL)).expect("receipt");
    assert_eq!(receipt["status"], "fail");
    assert!(
        receipt["why_failed"]
            .as_str()
            .unwrap()
            .contains("coverage_command_failed:coverage_claim_uncovered_code")
    );
    fs::remove_dir_all(root).expect("cleanup");
}

fn failing_executor(_root: &Path, _receipt: &Path) -> CoverageExecution {
    CoverageExecution {
        code: 2,
        stdout: String::new(),
        stderr:
            "info: cargo-llvm-cov currently setting cfg(coverage)\ncoverage_claim_uncovered_code\n"
                .to_string(),
        cache_mode: "coverage_authoritative_no_cache",
    }
}
