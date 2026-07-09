use super::{ImpactedRustTestsCommand, plan, receipt, runner};
use std::path::PathBuf;
use std::time::Instant;

#[test]
fn impacted_rust_tests_map_known_surfaces_to_focused_filters() {
    let plan = plan(&[
        PathBuf::from("validator/src/red/fixture/scheduler.rs"),
        PathBuf::from("validator/src/cli/live_loop/rust_tests/mod.rs"),
    ])
    .expect("plan");
    assert_eq!(plan.status, "pass");
    assert_eq!(
        plan.filters,
        vec![
            "cli::live_loop::rust_tests::".to_string(),
            "red::fixture::scheduler::".to_string()
        ]
    );
}

#[test]
fn impacted_rust_tests_fail_closed_for_unknown_or_empty_mapping() {
    let empty = plan(&[]).expect("empty plan");
    assert_eq!(empty.status, "fail");
    assert!(empty.failures[0].contains("changed_path_missing"));
    let unknown = plan(&[PathBuf::from("README.md")]).expect("unknown");
    assert_eq!(unknown.status, "fail");
    assert!(unknown.failures[0].contains("unknown_mapping"));
}

#[test]
fn impacted_rust_tests_reject_invalid_or_style_filter() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("rust-tests-or-filter");
    let result = runner::run_filters(&root, &["cli::live_loop::rust_tests::|red::".to_string()])
        .expect("or filter result");
    assert_eq!(result.status, "fail");
    assert_eq!(result.work_unit_count, 1);
    assert_eq!(
        result.commands[0]["failure_class"],
        "impacted_rust_tests_invalid_or_style_filter"
    );
}

#[test]
fn impacted_rust_tests_parse_zero_test_output_as_no_work() {
    let output = b"running 0 tests\n\ntest result: ok. 0 passed; 0 failed\n";
    assert_eq!(runner::executed_test_count(output), 0);
    let row = runner::row_from_output_for_test(true, output);
    assert_eq!(row["status"], "fail");
    assert_eq!(
        row["failure_class"],
        "impacted_rust_tests_zero_tests_executed"
    );
    let mixed = b"running 2 tests\nrunning 0 tests\nrunning 1 tests\n";
    assert_eq!(runner::executed_test_count(mixed), 3);
    let singular = b"running 1 test\n";
    assert_eq!(runner::executed_test_count(singular), 1);
}

#[test]
fn impacted_rust_tests_receipt_records_serial_runner_even_when_jobs_requested() {
    let command = ImpactedRustTestsCommand {
        receipt: "target/receipt.json".into(),
        jobs: Some(8),
        changed_paths: vec![PathBuf::from(
            "validator/src/cli/live_loop/rust_tests/mod.rs",
        )],
        run_tests: true,
    };
    let plan = plan(&command.changed_paths).expect("plan");
    let run = runner::result_for_test("pass", 2);
    let value = receipt(&command, &plan, Some(&run), Instant::now());
    assert_eq!(value["status"], "pass");
    assert_eq!(value["runner"]["requested_jobs"], "8");
    assert_eq!(value["runner"]["effective_worker_count"], 1);
    assert_eq!(value["runner"]["work_unit_count"], 1);
    assert_eq!(value["runner"]["total_executed_test_count"], 2);
    assert_eq!(
        value["runner"]["task_class"],
        "shared_authority_write_serial"
    );
}
