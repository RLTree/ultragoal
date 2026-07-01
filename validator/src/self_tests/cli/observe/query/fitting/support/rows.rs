use serde_json::json;

pub(super) fn insert_command(rows: &mut serde_json::Map<String, serde_json::Value>, command: &str) {
    let slug = slug(command);
    rows.insert(
        command.to_string(),
        base_row(
            command,
            &slug,
            None,
            format!("command:{command}"),
            fixture_slug(&slug),
        ),
    );
}

pub(super) fn insert_surface(rows: &mut serde_json::Map<String, serde_json::Value>, surface: &str) {
    let slug = slug(surface);
    let receipt_slug = format!("surface-{slug}");
    rows.insert(
        surface.to_string(),
        base_row(
            surface,
            &receipt_slug,
            Some(format!("surface.{}", surface.replace(' ', "."))),
            format!("surface:{surface}"),
            fixture_slug(&receipt_slug),
        ),
    );
}

pub(super) fn insert_operating(
    rows: &mut serde_json::Map<String, serde_json::Value>,
    kind: &str,
    name: &str,
) {
    insert_dimension_like(rows, kind, name);
}

pub(super) fn insert_dimension(
    rows: &mut serde_json::Map<String, serde_json::Value>,
    kind: &str,
    name: &str,
) {
    insert_dimension_like(rows, kind, name);
}

fn insert_dimension_like(
    rows: &mut serde_json::Map<String, serde_json::Value>,
    kind: &str,
    name: &str,
) {
    let slug = slug(name);
    let receipt_slug = format!("{kind}-{slug}");
    rows.insert(
        name.to_string(),
        base_row(
            name,
            &receipt_slug,
            Some(format!("observability.{kind}.{slug}")),
            format!("{kind}:{name}"),
            fixture_slug(&receipt_slug),
        ),
    );
}

fn base_row(
    name: &str,
    receipt_slug: &str,
    operation: Option<String>,
    owner: String,
    fixture_slug: String,
) -> serde_json::Value {
    let mut row = json!({
        "fitting_status": "fitted",
        "fitted_surfaces": fitted_surfaces(),
        "missing_surfaces": [],
        "validator_check_id": "full-local-observability-stack-integration-non-opaque-failure",
        "log_instrumentation": "log instrumentation fitted",
        "metric_instrumentation": "metric instrumentation fitted",
        "trace_instrumentation": "trace instrumentation fitted",
        "pass_stdout_contract": "pass stdout contract fitted",
        "fail_stdout_contract": "fail stdout contract fitted",
        "receipt_observability_binding": "receipt observability binding fitted",
        "focused_tests": ["observe_green_prove_and_query_helpers_are_typed"],
        "receipt_paths": [format!("validation_artifacts/observability/fitting/{receipt_slug}.json")],
        "live_query_proof_paths": query_paths(receipt_slug),
        "same_candidate_query_proof_paths": query_paths(receipt_slug),
        "red_fixtures": [format!("fixtures/red/observability/{fixture_slug}-red.json")],
        "green_fixtures": [format!("fixtures/green/observability/{fixture_slug}-green.json")],
        "tamper_fixtures": [format!("fixtures/tamper/observability/{fixture_slug}-tamper.json")],
        "current_owner_surface": owner,
        "next_unfitted_surface": "none",
        "claim_impact": "supports_gate_92_when_same_candidate"
    });
    if let Some(operation) = operation {
        row["operation"] = json!(operation);
    }
    row["name"] = json!(name);
    row
}

fn fitted_surfaces() -> [&'static str; 6] {
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
        format!("validation_artifacts/observability/fitting/{slug}-logs.json"),
        format!("validation_artifacts/observability/fitting/{slug}-metrics.json"),
        format!("validation_artifacts/observability/fitting/{slug}-traces.json"),
    ]
}

fn fixture_slug(slug: &str) -> String {
    slug.to_string()
}

fn slug(value: &str) -> String {
    value.replace(' ', "-")
}
