use super::*;
use serde_json::json;
use std::fs;

#[test]
fn red_fixture_report_parse_supports_default_and_alias() {
    let command = parse(&["red".into(), "fixture".into(), "report".into()])
        .expect("parse")
        .expect("command");
    assert_eq!(command.report, PathBuf::from(DEFAULT_REPORT));

    let alias = parse(&[
        "red-fixture-report".into(),
        "--red-report".into(),
        "custom.json".into(),
    ])
    .expect("alias parse")
    .expect("alias command");
    assert_eq!(alias.report, PathBuf::from("custom.json"));

    assert!(
        parse(&["source".into(), "audit".into()])
            .expect("other")
            .is_none()
    );
    assert!(
        parse(&[
            "red".into(),
            "fixture".into(),
            "report".into(),
            "--report".into()
        ])
        .expect_err("missing report")
        .contains("missing value for --report")
    );
    assert!(
        parse(&["red-fixture-report".into(), "--bogus".into()])
            .expect_err("unknown")
            .contains("unknown red fixture report argument")
    );
}

#[test]
fn red_fixture_report_command_generates_report_and_observability() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("red-report-command");
    fs::create_dir_all(root.join("fixtures/valid")).expect("valid fixtures");
    fs::create_dir_all(root.join("fixtures/red")).expect("red fixtures");
    fs::create_dir_all(root.join("templates")).expect("templates");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    )
    .expect("manifest");
    crate::json_boundary::write_json(
        &root.join("fixtures/valid/minimal-goal-run.json"),
        &json!({"claims":[]}),
    )
    .expect("base fixture");
    let packet = root.join("fixtures/red/missing-patch.json");
    crate::json_boundary::write_json(
        &packet,
        &json!({
            "expected_failure":{
                "check_id":"red-fixture-coverage",
                "error":"red_fixture_json_patch_missing"
            },
            "base_fixture_path":"fixtures/valid/minimal-goal-run.json"
        }),
    )
    .expect("red packet");
    crate::json_boundary::write_json(
        &root.join("templates/RED_FIXTURES.json"),
        &json!([{
            "id":"missing-patch",
            "packet_path":"fixtures/red/missing-patch.json",
            "expected_failure":{
                "check_id":"red-fixture-coverage",
                "error":"red_fixture_json_patch_missing"
            }
        }]),
    )
    .expect("red catalog");

    let report = PathBuf::from("validation_artifacts/ultragoal-audit/red-fixture-report.json");
    let command = RedReportCommand {
        report: report.clone(),
    };
    assert_eq!(run(&root, &command).expect("pass run"), 0);
    let generated_report =
        crate::json_boundary::read_json(&root.join(&report)).expect("generated report");
    assert_eq!(generated_report["status"], "pass");
    assert_eq!(
        generated_report["red_fixtures"]["missing-patch"]["observed_error"],
        "red_fixture_json_patch_missing"
    );
    let receipt_path = root.join("validation_artifacts/observability/red-fixture-report.json");
    let pass = crate::json_boundary::read_json(&receipt_path).expect("pass receipt");
    assert_eq!(pass["status"], "pass");
    assert_eq!(pass["event"]["command"], "ultragoal red");
    assert_eq!(pass["event"]["subcommand"], "fixture report");
    assert_eq!(pass["operation"], "red_fixture.report");
    assert_eq!(
        pass["event"]["cache_mode"],
        "red_report_standalone_generate"
    );
    assert_eq!(
        pass["event"]["resource_measurement_status"],
        "standalone_red_report_generation_wall_time_only"
    );
    assert_eq!(pass["event"]["worker_count"], 1);
    assert_eq!(pass["event"]["task_count"], 1);
    assert_eq!(
        pass["event"]["repair_anchor_before"],
        "red_fixture_report_command_start"
    );
    let pass_stdout = crate::cli::audit::red_report_stdout_contract_for_test(&pass);
    assert_eq!(pass_stdout.len(), 1);
    assert!(pass_stdout[0].contains("operation=red_fixture.report"));

    let dispatch = crate::parse_command(&[
        "red".to_string(),
        "fixture".to_string(),
        "report".to_string(),
        "--report".to_string(),
        report.display().to_string(),
    ])
    .expect("parse dispatch");
    let code = crate::command_run::run_with_exit_code(crate::Args {
        root: root.clone(),
        command: dispatch,
    })
    .expect("dispatch run");
    assert_eq!(code, 0);

    crate::json_boundary::write_json(
        &packet,
        &json!({
            "expected_failure":{
                "check_id":"red-fixture-coverage",
                "error":"not_the_observed_failure"
            },
            "base_fixture_path":"fixtures/valid/minimal-goal-run.json"
        }),
    )
    .expect("failing red packet");
    assert_eq!(run(&root, &command).expect("fail run"), 1);
    let failed_report =
        crate::json_boundary::read_json(&root.join(&report)).expect("failed report");
    assert_eq!(failed_report["status"], "fail");
    assert_eq!(
        failed_report["red_fixtures"]["missing-patch"]["observed_error"],
        "red_fixture_json_patch_missing"
    );
    let fail = crate::json_boundary::read_json(&receipt_path).expect("fail receipt");
    assert_eq!(fail["status"], "fail");
    assert_eq!(fail["event"]["failure_class"], "red_fixture_report_failure");
    assert!(
        fail["why_failed"]
            .as_str()
            .unwrap_or_default()
            .contains("missing-patch")
    );
    let fail_stdout = crate::cli::audit::red_report_stdout_contract_for_test(&fail);
    assert!(
        fail_stdout
            .iter()
            .any(|line| line.contains("failed_check=red-fixture-report-observability-binding"))
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn red_fixture_report_command_rejects_unowned_claim_report_path() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("red-report-absolute-path");
    let command = RedReportCommand {
        report: root.join("validation_artifacts/ultragoal-audit/red-fixture-report.json"),
    };
    let err = run(&root, &command).expect_err("absolute report rejected");
    assert!(err.contains("root-relative claim artifact path"), "{err}");
    assert!(!command.report.exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn red_fixture_report_command_rejects_non_canonical_report_basename() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("red-report-basename");
    let command = RedReportCommand {
        report: PathBuf::from("validation_artifacts/ultragoal-audit/report.json"),
    };
    let err = run(&root, &command).expect_err("non-canonical basename rejected");
    assert!(err.contains("red-fixture-report.json"), "{err}");
    assert!(!root.join(&command.report).exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn red_fixture_report_command_propagates_observability_write_errors() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("red-report-no-candidate");
    let command = RedReportCommand {
        report: PathBuf::from(DEFAULT_REPORT),
    };
    let err = run(&root, &command).expect_err("missing candidate blocks observation");
    assert!(err.contains("plugin-manifest-draft.json"), "{err}");
    let _ = fs::remove_dir_all(root);
}
