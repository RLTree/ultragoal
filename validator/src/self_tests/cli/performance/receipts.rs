use crate::cli::performance::receipt::surface_value_failures;
use crate::cli::performance::types::PERFORMANCE_RECEIPT_SCHEMA;
use serde_json::{Value, json};

fn valid_fail() -> Value {
    json!({
        "schema": PERFORMANCE_RECEIPT_SCHEMA,
        "status": "fail",
        "claim_ceiling": "withheld_or_blocked",
        "command": {"argv": ["ultragoal", "performance", "prove"]},
        "budget": {"class": "strict_local"},
        "digests": {"candidate": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},
        "cache": {"mode": "disabled"},
        "concurrency": {"worker_count": 1},
        "telemetry": {"wall_clock_ms": 1},
        "failure": {"check_id": "cli-performance-latency-speed-iteration-fitness"},
        "blocked_claim_classes": ["completion"]
    })
}

#[test]
fn accepts_fail_closed_receipt_surface() {
    assert!(surface_value_failures(&valid_fail()).is_empty());
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

    let mut fake_pass = valid_fail();
    fake_pass["status"] = json!("pass");
    fake_pass["claim_ceiling"] = json!("withheld_or_blocked");
    assert!(
        surface_value_failures(&fake_pass)
            .contains(&"cli_performance_pass_without_positive_claim_ceiling".to_string())
    );

    let mut unsupported_fail = valid_fail();
    unsupported_fail["blocked_claim_classes"] = json!([]);
    assert!(
        surface_value_failures(&unsupported_fail)
            .contains(&"cli_performance_fail_without_blocked_claims".to_string())
    );
}
