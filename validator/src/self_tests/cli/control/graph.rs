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

fn labels() -> [&'static str; 27] {
    [
        "source_audit",
        "red_fixture_report",
        "coverage",
        "cli_performance",
        "final_packet",
        "registry_exposure",
        "fit_repo",
        "product_fitness",
        "product_journey",
        "standards_gardener",
        "install_audit",
        "cache_audit",
        "rust_toolchain",
        "rust_fast",
        "rust_standard",
        "rust_release",
        "rust_clean_proof",
        "rust_watch",
        "rust_memory",
        "rust_dependency",
        "rust_coverage",
        "rust_workspace_topology",
        "gc_plan",
        "gc_dry_run",
        "gc_apply",
        "gc_verify",
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
