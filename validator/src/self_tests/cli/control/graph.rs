use serde_json::{Value, json};

pub(crate) fn control(candidate: &str, operation: &str, fail_closed: bool) -> Value {
    let status = if fail_closed { "fail" } else { "pass" };
    let item_failures = if fail_closed {
        json!(["status_not_pass"])
    } else {
        json!([])
    };
    let operation_failures = if fail_closed {
        json!(["missing_same_surface_proof"])
    } else {
        json!([])
    };
    json!({
        "candidate_digest": candidate,
        "evaluation_mode": "production_dereferenced",
        "operation": operation,
        "operation_failures": operation_failures,
        "items": labels().into_iter().map(|label| item(label, candidate, status, &item_failures)).collect::<Vec<_>>()
    })
}

fn labels() -> [&'static str; 7] {
    [
        "red_fixture_report",
        "coverage",
        "cli_performance",
        "final_packet",
        "registry_exposure",
        "source_audit",
        "transactional_finalization",
    ]
}

fn item(label: &str, candidate: &str, status: &str, failures: &Value) -> Value {
    json!({
        "label": label,
        "path": format!("validation_artifacts/cli-control-test/{label}.json"),
        "exists": true,
        "digest": candidate,
        "schema": format!("harness-ultragoal.{label}.v1"),
        "status": status,
        "candidate_digest": candidate,
        "same_candidate": true,
        "failures": failures
    })
}
