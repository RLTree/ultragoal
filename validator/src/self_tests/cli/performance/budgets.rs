use crate::cli::performance::types::{
    BudgetClass, PERFORMANCE_COMMANDS, PERFORMANCE_RECEIPT_SCHEMA, PerformanceOperation,
};
use crate::cli::performance::{PerformanceCommand, parse, receipt, run};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("validator has repo parent")
        .to_path_buf()
}

fn temp_receipt(root: &Path) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    root.join("target")
        .join(format!("cli-performance-test-{stamp}.json"))
}

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

#[test]
fn parses_performance_command_variants_and_budget_aliases() {
    assert!(parse(&args(&["unknown"])).expect("parse ok").is_none());

    let prove = parse(&args(&["performance", "prove"]))
        .expect("parse prove")
        .expect("performance command");
    assert_eq!(prove.operation, PerformanceOperation::Prove);
    assert_eq!(prove.class, BudgetClass::StrictLocal);
    assert!(prove.receipt.is_none());

    let verify = parse(&args(&[
        "performance",
        "verify",
        "--class",
        "repair-loop",
        "--receipt",
        "target/perf.json",
    ]))
    .expect("parse verify")
    .expect("performance command");
    assert_eq!(verify.operation, PerformanceOperation::Verify);
    assert_eq!(verify.class, BudgetClass::RepairLoop);
    assert_eq!(verify.receipt, Some(PathBuf::from("target/perf.json")));

    let verify_default = parse(&args(&["performance", "verify"]))
        .expect("parse verify default")
        .expect("performance verify");
    assert_eq!(verify_default.operation, PerformanceOperation::Verify);
    assert_eq!(verify_default.class, BudgetClass::Focused);

    let budgets = parse(&args(&["performance", "budgets"]))
        .expect("parse budgets")
        .expect("performance command");
    assert_eq!(budgets.operation, PerformanceOperation::Budgets);
    assert_eq!(budgets.class, BudgetClass::Instant);

    let self_prove = parse(&args(&[
        "self",
        "performance",
        "prove",
        "--class",
        "external_live",
    ]))
    .expect("parse self prove")
    .expect("performance command");
    assert_eq!(self_prove.operation, PerformanceOperation::SelfProve);
    assert_eq!(self_prove.class, BudgetClass::ExternalLive);

    let err = parse(&args(&["performance", "prove", "--class", "slow"]))
        .expect_err("invalid budget fails");
    assert!(err.contains("invalid --class"));
}

#[test]
fn budget_classes_have_closed_ids_and_thresholds() {
    let rows = [
        (BudgetClass::Instant, "instant", 2_000, Some(500)),
        (BudgetClass::Interactive, "interactive", 5_000, Some(1_000)),
        (BudgetClass::Focused, "focused", 15_000, Some(5_000)),
        (BudgetClass::RepairLoop, "repair_loop", 30_000, Some(10_000)),
        (BudgetClass::StrictLocal, "strict_local", 60_000, None),
        (
            BudgetClass::StrictFixtures,
            "strict_fixtures",
            120_000,
            None,
        ),
        (
            BudgetClass::StrictCoverage,
            "strict_coverage",
            300_000,
            None,
        ),
        (BudgetClass::StrictFinal, "strict_final", 600_000, None),
        (BudgetClass::ExternalLive, "external_live", 30_000, None),
    ];
    for (class, id, cold, warm) in rows {
        assert_eq!(class.id(), id);
        assert_eq!(BudgetClass::from_str(id), Some(class));
        assert_eq!(BudgetClass::from_str(&id.replace('_', "-")), Some(class));
        assert_eq!(class.cold_p95_ms(), cold);
        assert_eq!(class.warm_p95_ms(), warm);
    }
    assert_eq!(BudgetClass::from_str("unknown"), None);

    assert_eq!(PerformanceOperation::Prove.id(), "performance_prove");
    assert_eq!(PerformanceOperation::Verify.id(), "performance_verify");
    assert_eq!(PerformanceOperation::Budgets.id(), "performance_budgets");
    assert_eq!(
        PerformanceOperation::SelfProve.id(),
        "self_performance_prove"
    );
    assert!(PERFORMANCE_COMMANDS.contains(&"ultragoal performance prove"));
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
    assert_eq!(value["budget"]["threshold_ms"], 30_000);
    assert_eq!(value["cache"]["mode"], "disabled");
    assert_eq!(value["concurrency"]["worker_count"], 1);
    assert_eq!(value["telemetry"]["wall_clock_ms"], 123);
    assert_eq!(value["external_probe_policy"]["live_probe_class"], true);
    assert_eq!(value["external_probe_policy"]["timeout_ms"], 30_000);
    assert_eq!(value["failure"], serde_json::Value::Null);
    assert_eq!(
        value["supported_claim_classes"]
            .as_array()
            .expect("supported claims")
            .iter()
            .filter_map(|claim| claim.as_str())
            .collect::<Vec<_>>(),
        vec!["routine_usability"]
    );
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
        class: BudgetClass::Focused,
    };
    assert_eq!(run(&root, &command).expect("run succeeds"), 0);
    let value = crate::json_boundary::read_json(&path).expect("read written receipt");
    assert_eq!(value["command"]["name"], "performance_prove");
    assert_eq!(value["budget"]["class"], "focused");
    assert_eq!(value["output_size_metrics"]["receipt_count_written"], 1);
    assert_eq!(value["status"], "pass");

    let no_write = PerformanceCommand {
        operation: PerformanceOperation::Budgets,
        receipt: None,
        class: BudgetClass::StrictLocal,
    };
    assert_eq!(run(&root, &no_write).expect("run prints receipt"), 0);
}
