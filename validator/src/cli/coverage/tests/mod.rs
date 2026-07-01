use super::*;
use serde_json::{Value, json};
use std::fs;

mod execution_edges;
mod validation;

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
    assert!(parse(&["source".into(), "audit".into()]).unwrap().is_none());
    assert!(
        parse(&["coverage".into(), "prove".into(), "--bad".into()])
            .expect_err("unknown")
            .contains("unknown coverage prove argument")
    );
}

#[test]
fn coverage_prove_command_writes_pass_and_fail_observability() {
    let root = crate::self_tests::boundaries::support::temp_root("coverage-prove-command");
    write_coverage_root(&root, 100.0, json!([]));
    let command = CoverageCommand {
        receipt: PathBuf::from(COVERAGE_RECEIPT_REL),
        jobs: Some(4),
        validate_existing: true,
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

    write_coverage_root(&root, 99.0, json!([{"path":"src/lib.rs"}]));
    assert_eq!(run(&root, &command).expect("fail run"), 1);
    let fail =
        crate::json_boundary::read_json(&root.join(OBSERVABILITY_RECEIPT_REL)).expect("fail");
    assert_eq!(fail["status"], "fail");
    assert_eq!(fail["event"]["failure_class"], "coverage_prove_failure");
    assert!(fail["supported_claims"].as_array().unwrap().is_empty());
    assert!(
        stdout::contract(&fail)
            .iter()
            .any(|line| line.contains("failed_check=coverage-prove-observability-binding"))
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn coverage_prove_records_authoritative_command_failure() {
    let root = crate::self_tests::boundaries::support::temp_root("coverage-prove-executor");
    write_coverage_root(&root, 100.0, json!([]));
    let command = CoverageCommand {
        receipt: PathBuf::from(COVERAGE_RECEIPT_REL),
        jobs: None,
        validate_existing: false,
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

fn write_coverage_root(root: &Path, percent: f64, uncovered: Value) {
    for rel in [".harness", "src", "validation_artifacts/coverage"] {
        fs::create_dir_all(root.join(rel)).expect("dir");
    }
    crate::json_boundary::write_json(&root.join("plugin-manifest-draft.json"), &json!({}))
        .expect("manifest");
    fs::write(root.join("src/lib.rs"), "pub fn answer() -> usize { 42 }\n").expect("source");
    fs::write(
        root.join(".harness/coverage-command"),
        "bash .harness/run-coverage.sh\n",
    )
    .expect("coverage command");
    let manifest = json!({
        "schema": "harness-ultragoal.coverage-manifest.v1",
        "required_target_paths": ["src/lib.rs"],
        "changed_file_coupling_policy": {
            "required": true,
            "changed_files": ["src/lib.rs"]
        },
        "required_measured_dimensions_per_root": [{
            "root": "src",
            "dimensions": ["line"]
        }],
        "source_discovery_rules": {"ignore": []},
        "exclusions": []
    });
    crate::json_boundary::write_json(&root.join(".harness/coverage-manifest.json"), &manifest)
        .expect("coverage manifest");
    let report = json!({"data":[{"totals":{"lines":{"percent":percent}}}]});
    crate::json_boundary::write_json(
        &root.join("validation_artifacts/coverage/llvm-cov-full.json"),
        &report,
    )
    .expect("report");
    let candidate = crate::package::inventory::package_digest(root).expect("candidate");
    let receipt = coverage_receipt(root, &manifest, percent, uncovered, &candidate);
    crate::json_boundary::write_json(&root.join(COVERAGE_RECEIPT_REL), &receipt)
        .expect("coverage receipt");
}

fn coverage_receipt(
    root: &Path,
    manifest: &Value,
    percent: f64,
    uncovered: Value,
    candidate: &str,
) -> Value {
    let report = root.join("validation_artifacts/coverage/llvm-cov-full.json");
    json!({
        "schema": "harness-ultragoal.coverage-receipt.v1",
        "claim_id": "CLAIM-001",
        "command": "bash .harness/run-coverage.sh",
        "tool": "cargo-llvm-cov",
        "source_tree_digest": crate::claim_semantics::coverage::digests::source_tree_digest(root, manifest).expect("source"),
        "coverage_manifest_digest": crate::digest::file(&root.join(".harness/coverage-manifest.json")).expect("manifest digest"),
        "coverage_command_digest": crate::digest::file(&root.join(".harness/coverage-command")).expect("command digest"),
        "changed_files_digest": crate::claim_semantics::coverage::digests::changed_files_digest(root, manifest).expect("changed"),
        "tool_version": "cargo-llvm-cov test",
        "workspace_root": root.canonicalize().expect("canonical root").to_string_lossy(),
        "target_revision": {"kind": "package_digest", "value": candidate},
        "command_started_at": "2026-01-01T00:00:00Z",
        "command_completed_at": "2026-01-01T00:00:01Z",
        "command_exit": 0,
        "machine_readable_report": {
            "path": "validation_artifacts/coverage/llvm-cov-full.json",
            "digest": crate::digest::file(&report).expect("report digest")
        },
        "generated_by": "coverage-command",
        "percent_source": "machine_readable_report",
        "target_paths": ["src/lib.rs"],
        "measured_dimensions": ["line"],
        "coverage": {
            "percent": percent,
            "floor_percent": 100,
            "policy": "100_percent_required"
        },
        "uncovered_records": uncovered,
        "exclusions": [],
        "generated_at": "2026-01-01T00:00:01Z",
        "claim_ceiling": if percent == 100.0 {"supports_complete_coverage_claim"} else {"withheld_or_blocked"},
        "supported_claim_classes": if percent == 100.0 {json!(["complete_coverage"])} else {json!([])},
        "blocked_claim_classes": [
            "completion",
            "package_readiness",
            "review_readiness",
            "release_readiness",
            "final_packet_correctness",
            "update_goal_eligibility",
            "app_registry_or_reviewer_exposure"
        ]
    })
}
