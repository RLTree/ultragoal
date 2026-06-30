use crate::cli::performance::receipt::{same_candidate_pass_failures, surface_value_failures};
use crate::cli::performance::types::PERFORMANCE_RECEIPT_SCHEMA;
use serde_json::{Value, json};

fn valid_fail() -> Value {
    json!({
        "schema": PERFORMANCE_RECEIPT_SCHEMA,
        "status": "fail",
        "claim_ceiling": "withheld_or_blocked",
        "command": {"argv": ["ultragoal", "performance", "prove"]},
        "budget": strict_budget(),
        "digests": {"candidate": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},
        "cache": {"mode": "disabled"},
        "concurrency": {"worker_count": 1},
        "telemetry": {"wall_clock_ms": 1},
        "failure": {"check_id": "cli-performance-latency-speed-iteration-fitness"},
        "blocked_claim_classes": ["completion"]
    })
}

fn valid_pass() -> Value {
    let mut value = valid_fail();
    value["status"] = json!("pass");
    value["claim_ceiling"] = json!("performance_proven");
    value["failure"] = Value::Null;
    value["blocked_claim_classes"] = json!([]);
    value["supported_claim_classes"] = json!(["routine_usability"]);
    value
}

fn strict_pass(candidate: &str) -> Value {
    json!({
        "schema": PERFORMANCE_RECEIPT_SCHEMA,
        "status": "pass",
        "claim_ceiling": "performance_proven",
        "command": {"argv": ["ultragoal", "performance", "prove"]},
        "budget": strict_budget(),
        "digests": {"candidate": candidate},
        "cache": {"mode": "disabled", "no_cache_mode_result": "executed_without_cache"},
        "concurrency": {"worker_count": 1},
        "telemetry": {"wall_clock_ms": 1},
        "performance_regression": {"status": "pass"},
        "failure": null,
        "blocked_claim_classes": [],
        "supported_claim_classes": ["routine_usability"]
    })
}

fn strict_budget() -> Value {
    json!({
        "class": "strict_local",
        "cold_p95_ms": 60_000,
        "warm_p95_ms": null,
        "target_ms": 60_000,
        "hard_ceiling_ms": 180_000,
        "threshold_ms": 60_000
    })
}

#[test]
fn accepts_fail_closed_receipt_surface() {
    assert!(surface_value_failures(&valid_fail()).is_empty());
    assert!(surface_value_failures(&valid_pass()).is_empty());
}

#[test]
fn rejects_wrong_schema_missing_fields_and_claim_theater() {
    let mut wrong_schema = valid_fail();
    wrong_schema["schema"] = json!("other");
    assert!(
        surface_value_failures(&wrong_schema)
            .contains(&"cli_performance_receipt_wrong_schema".to_string())
    );

    let mut missing = valid_fail();
    missing["telemetry"]
        .as_object_mut()
        .expect("telemetry object")
        .remove("wall_clock_ms");
    assert!(
        surface_value_failures(&missing)
            .iter()
            .any(|failure| failure == "cli_performance_receipt_missing:/telemetry/wall_clock_ms")
    );

    let mut fake_pass = valid_pass();
    fake_pass["claim_ceiling"] = json!("withheld_or_blocked");
    assert!(
        surface_value_failures(&fake_pass)
            .contains(&"cli_performance_pass_without_positive_claim_ceiling".to_string())
    );

    let mut blocked_pass = valid_pass();
    blocked_pass["blocked_claim_classes"] = json!(["completion"]);
    assert!(
        surface_value_failures(&blocked_pass)
            .contains(&"cli_performance_pass_with_blocked_claims".to_string())
    );

    let mut missing_failure_check = valid_fail();
    missing_failure_check["failure"]
        .as_object_mut()
        .expect("failure object")
        .remove("check_id");
    assert!(
        surface_value_failures(&missing_failure_check)
            .contains(&"cli_performance_receipt_missing:/failure/check_id".to_string())
    );

    let mut unsupported_fail = valid_fail();
    unsupported_fail["blocked_claim_classes"] = json!([]);
    assert!(
        surface_value_failures(&unsupported_fail)
            .contains(&"cli_performance_fail_without_blocked_claims".to_string())
    );

    let mut legacy_budget = valid_pass();
    legacy_budget["budget"]["class"] = json!("focused");
    assert!(
        surface_value_failures(&legacy_budget)
            .iter()
            .any(|failure| failure == "cli_performance_receipt_noncanonical_budget_class:focused")
    );

    let mut wrong_target = valid_pass();
    wrong_target["budget"]["hard_ceiling_ms"] = json!(60_000);
    assert!(surface_value_failures(&wrong_target).iter().any(|failure| {
        failure == "cli_performance_receipt_budget_hard_ceiling_mismatch:strict_local"
    }));
}

#[test]
fn strict_surface_validation_rejects_stale_or_placeholder_performance_proof() {
    let stale = crate::self_tests::boundaries::support::sha('a');
    let current = crate::self_tests::boundaries::support::sha('b');
    let mut receipt = strict_pass(&stale);
    receipt["telemetry"]["wall_clock_ms"] = json!(0);
    receipt["performance_regression"]["status"] = json!("missing_baseline");
    receipt["cache"]["mode"] = json!("hidden");
    receipt["failure"] = json!({"check_id": "still-blocked"});
    receipt["supported_claim_classes"] = json!(["routine_usability", "update_goal_eligibility"]);
    let failures = same_candidate_pass_failures(&receipt, &current);
    for expected in [
        "cli_performance_receipt_candidate_digest_mismatch",
        "cli_performance_pass_has_failure",
        "cli_performance_receipt_placeholder_wall_clock",
        "cli_performance_receipt_regression_not_pass",
        "cli_performance_receipt_cache_honesty_missing",
        "cli_performance_receipt_update_goal_overclaim",
    ] {
        assert!(
            failures.iter().any(|failure| failure.contains(expected)),
            "{expected}: {failures:?}"
        );
    }
    let green_failures = same_candidate_pass_failures(&strict_pass(&current), &current);
    assert!(green_failures.is_empty(), "{green_failures:?}");
}
