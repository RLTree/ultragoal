use serde_json::{Value, json};

pub(super) fn observable_row(command: &str) -> Value {
    let slug = slug(command);
    json!({
        "observability_status": "observable",
        "observed_surfaces": observed_surfaces(),
        "missing_surfaces": [],
        "validator_check_id": crate::audit::observability::LAW,
        "log_instrumentation": "log instrumentation observable",
        "metric_instrumentation": "metric instrumentation observable",
        "trace_instrumentation": "trace instrumentation observable",
        "pass_stdout_contract": "pass stdout contract observable",
        "fail_stdout_contract": "fail stdout contract observable",
        "receipt_observability_binding": "receipt observability binding observable",
        "focused_tests": ["observability_registry_accepts_fully_observable_inventory"],
        "receipt_paths": [format!("validation_artifacts/observability/command-roundtrip/{slug}.json")],
        "live_query_proof_paths": query_paths(&slug),
        "same_candidate_query_proof_paths": query_paths(&slug),
        "red_fixtures": [format!("fixtures/red/observability/{slug}-red.json")],
        "green_fixtures": [format!("fixtures/green/observability/{slug}-green.json")],
        "tamper_fixtures": [format!("fixtures/tamper/observability/{slug}-tamper.json")],
        "current_owner_surface": format!("command:{command}"),
        "next_unobservable_surface": "none",
        "claim_impact": "supports_observability_gate_when_same_candidate"
    })
}

pub(super) fn observable_surface_row(surface: &str) -> Value {
    let slug = slug(surface);
    let receipt_slug = format!("surface-{slug}");
    json!({
        "observability_status": "observable",
        "observed_surfaces": observed_surfaces(),
        "missing_surfaces": [],
        "operation": format!("surface.{}", surface.replace(' ', ".")),
        "validator_check_id": crate::audit::observability::LAW,
        "log_instrumentation": "log instrumentation observable",
        "metric_instrumentation": "metric instrumentation observable",
        "trace_instrumentation": "trace instrumentation observable",
        "pass_stdout_contract": "pass stdout contract observable",
        "fail_stdout_contract": "fail stdout contract observable",
        "receipt_observability_binding": "receipt observability binding observable",
        "focused_tests": ["observability_registry_accepts_fully_observable_inventory"],
        "receipt_paths": [format!("validation_artifacts/observability/command-roundtrip/{receipt_slug}.json")],
        "live_query_proof_paths": query_paths(&receipt_slug),
        "same_candidate_query_proof_paths": query_paths(&receipt_slug),
        "red_fixtures": [format!("fixtures/red/observability/{receipt_slug}-red.json")],
        "green_fixtures": [format!("fixtures/green/observability/{receipt_slug}-green.json")],
        "tamper_fixtures": [format!("fixtures/tamper/observability/{receipt_slug}-tamper.json")],
        "current_owner_surface": format!("surface:{surface}"),
        "next_unobservable_surface": "none",
        "claim_impact": "supports_observability_gate_when_same_candidate"
    })
}

pub(super) fn observable_operating_row(kind: &str, name: &str) -> Value {
    let slug = slug(name);
    let receipt_slug = format!("{kind}-{slug}");
    json!({
        "observability_status": "observable",
        "observed_surfaces": observed_surfaces(),
        "missing_surfaces": [],
        "operation": format!("observability.{kind}.{slug}"),
        "validator_check_id": crate::audit::observability::LAW,
        "log_instrumentation": "log instrumentation observable",
        "metric_instrumentation": "metric instrumentation observable",
        "trace_instrumentation": "trace instrumentation observable",
        "pass_stdout_contract": "pass stdout contract observable",
        "fail_stdout_contract": "fail stdout contract observable",
        "receipt_observability_binding": "receipt observability binding observable",
        "focused_tests": ["observability_registry_accepts_fully_observable_inventory"],
        "receipt_paths": [format!("validation_artifacts/observability/command-roundtrip/{receipt_slug}.json")],
        "live_query_proof_paths": query_paths(&receipt_slug),
        "same_candidate_query_proof_paths": query_paths(&receipt_slug),
        "red_fixtures": [format!("fixtures/red/observability/{receipt_slug}-red.json")],
        "green_fixtures": [format!("fixtures/green/observability/{receipt_slug}-green.json")],
        "tamper_fixtures": [format!("fixtures/tamper/observability/{receipt_slug}-tamper.json")],
        "current_owner_surface": format!("{kind}:{name}"),
        "next_unobservable_surface": "none",
        "claim_impact": "supports_observability_gate_when_same_candidate"
    })
}

fn observed_surfaces() -> [&'static str; 6] {
    [
        "log instrumentation",
        "metric instrumentation",
        "trace instrumentation",
        "pass stdout contract",
        "fail stdout contract",
        "receipt observability binding",
    ]
}

fn query_paths(slug: &str) -> [String; 3] {
    [
        format!("validation_artifacts/observability/command-roundtrip/{slug}-logs.json"),
        format!("validation_artifacts/observability/command-roundtrip/{slug}-metrics.json"),
        format!("validation_artifacts/observability/command-roundtrip/{slug}-traces.json"),
    ]
}

fn slug(command: &str) -> String {
    command.replace(' ', "-")
}
