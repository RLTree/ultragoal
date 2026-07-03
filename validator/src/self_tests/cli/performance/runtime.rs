use crate::cli::performance::types::{
    BudgetClass, PERFORMANCE_RECEIPT_SCHEMA, PerformanceOperation,
};
use crate::cli::performance::{PerformanceCommand, receipt, run};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("validator has repo parent")
        .to_path_buf()
}

fn temp_receipt(_root: &Path) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    PathBuf::from("target").join(format!("cli-performance-test-{stamp}.json"))
}

#[test]
fn receipt_is_fail_closed_and_surface_bound() {
    let root = repo_root();
    let command = PerformanceCommand {
        operation: PerformanceOperation::SelfProve,
        receipt: Some(PathBuf::from("target/perf.json")),
        class: BudgetClass::ExternalLive,
    };
    let value = receipt(&root, &command, 123).expect("receipt builds");
    assert_eq!(value["schema"], PERFORMANCE_RECEIPT_SCHEMA);
    assert_eq!(value["status"], "pass");
    assert_eq!(value["claim_ceiling"], "performance_proven");
    assert_eq!(value["command"]["name"], "self_performance_prove");
    assert_eq!(value["budget"]["class"], "external_live");
    assert_eq!(value["budget"]["target_ms"], 30_000);
    assert_eq!(value["budget"]["hard_ceiling_ms"], 30_000);
    assert_eq!(value["budget"]["threshold_ms"], 30_000);
    assert_eq!(value["cache"]["mode"], "disabled");
    assert_eq!(
        value["concurrency"]["worker_count"],
        crate::scheduler::SchedulerConfig::default_jobs()
    );
    assert_eq!(
        value["concurrency"]["isolation_namespace"],
        "scheduler_default_available_parallelism_minus_one_no_shared_artifact_writes"
    );
    assert_eq!(value["telemetry"]["wall_clock_ms"], 123);
    assert_eq!(value["external_probe_policy"]["live_probe_class"], true);
    assert_eq!(value["external_probe_policy"]["timeout_ms"], 30_000);
    assert_eq!(value["failure"], serde_json::Value::Null);
    let claims = value["supported_claim_classes"]
        .as_array()
        .expect("supported claims")
        .iter()
        .filter_map(|claim| claim.as_str())
        .collect::<Vec<_>>();
    assert_eq!(claims, vec!["routine_usability"]);
    for ptr in [
        "/digests/candidate",
        "/digests/cli_binary",
        "/digests/schema_catalog",
        "/digests/law_graph",
        "/digests/standards",
        "/digests/fixture_catalog",
        "/digests/source_obligation",
        "/digests/config",
        "/input_size_metrics/file_count_scanned",
        "/input_size_metrics/fixture_count_executed",
        "/performance_regression/baseline_machine_class",
    ] {
        assert!(value.pointer(ptr).is_some(), "missing {ptr}");
    }

    let over_budget =
        receipt(&root, &command, BudgetClass::ExternalLive.cold_p95_ms() + 1).expect("receipt");
    assert_eq!(over_budget["status"], "fail");
    assert_eq!(over_budget["claim_ceiling"], "withheld_or_blocked");
    assert_eq!(
        over_budget["failure"]["id"],
        "cli_performance_budget_exceeded"
    );
    assert!(
        over_budget["blocked_claim_classes"]
            .as_array()
            .expect("blocked claims")
            .iter()
            .any(|claim| claim.as_str() == Some("update_goal_eligibility"))
    );
}

#[test]
fn run_writes_receipt_and_returns_typed_exit_code() {
    let root = repo_root();
    let path = temp_receipt(&root);
    let command = PerformanceCommand {
        operation: PerformanceOperation::Prove,
        receipt: Some(path.clone()),
        class: BudgetClass::FocusedRepair,
    };
    assert_eq!(run(&root, &command).expect("run succeeds"), 0);
    let value = crate::json_boundary::read_json(&root.join(&path)).expect("read written receipt");
    assert_eq!(value["command"]["name"], "performance_prove");
    assert_eq!(value["budget"]["class"], "focused_repair");
    assert_eq!(value["output_size_metrics"]["receipt_count_written"], 1);
    assert_eq!(value["status"], "pass");

    let no_write = PerformanceCommand {
        operation: PerformanceOperation::Budgets,
        receipt: None,
        class: BudgetClass::StrictLocal,
    };
    assert_eq!(run(&root, &no_write).expect("run prints receipt"), 0);
}
