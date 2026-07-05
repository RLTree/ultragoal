use crate::cli::performance::parse;
use crate::cli::performance::types::{BudgetClass, PERFORMANCE_COMMANDS, PerformanceOperation};
use std::path::PathBuf;

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
        "validation_artifacts/performance/perf.json",
    ]))
    .expect("parse verify")
    .expect("performance command");
    assert_eq!(verify.operation, PerformanceOperation::Verify);
    assert_eq!(verify.class, BudgetClass::StandardSourceLocal);
    assert_eq!(
        verify.receipt,
        Some(PathBuf::from("validation_artifacts/performance/perf.json"))
    );

    let verify_default = parse(&args(&["performance", "verify"]))
        .expect("parse verify default")
        .expect("performance verify");
    assert_eq!(verify_default.operation, PerformanceOperation::Verify);
    assert_eq!(verify_default.class, BudgetClass::FocusedRepair);

    let budgets = parse(&args(&["performance", "budgets"]))
        .expect("parse budgets")
        .expect("performance command");
    assert_eq!(budgets.operation, PerformanceOperation::Budgets);
    assert_eq!(budgets.class, BudgetClass::HotEditCheck);

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
        (
            BudgetClass::HotEditCheck,
            "hot_edit_check",
            5_000,
            Some(5_000),
            5_000,
        ),
        (
            BudgetClass::FocusedRepair,
            "focused_repair",
            15_000,
            Some(5_000),
            15_000,
        ),
        (
            BudgetClass::StandardSourceLocal,
            "standard_source_local",
            30_000,
            Some(10_000),
            60_000,
        ),
        (
            BudgetClass::StrictLocal,
            "strict_local",
            60_000,
            None,
            180_000,
        ),
        (
            BudgetClass::StrictFixtures,
            "strict_fixtures",
            60_000,
            None,
            180_000,
        ),
        (
            BudgetClass::StrictCoverage,
            "strict_coverage",
            60_000,
            None,
            180_000,
        ),
        (
            BudgetClass::StrictFinal,
            "strict_final",
            60_000,
            None,
            180_000,
        ),
        (
            BudgetClass::ExternalLive,
            "external_live",
            30_000,
            None,
            30_000,
        ),
    ];
    for (class, id, cold, warm, hard_ceiling) in rows {
        assert_eq!(class.id(), id);
        assert_eq!(BudgetClass::from_str(id), Some(class));
        assert_eq!(BudgetClass::from_str(&id.replace('_', "-")), Some(class));
        assert_eq!(class.cold_p95_ms(), cold);
        assert_eq!(class.warm_p95_ms(), warm);
        assert_eq!(class.hard_ceiling_ms(), hard_ceiling);
    }
    assert_eq!(
        BudgetClass::from_str("instant"),
        Some(BudgetClass::HotEditCheck)
    );
    assert_eq!(
        BudgetClass::from_str("interactive"),
        Some(BudgetClass::HotEditCheck)
    );
    assert_eq!(
        BudgetClass::from_str("repair_loop"),
        Some(BudgetClass::StandardSourceLocal)
    );
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
