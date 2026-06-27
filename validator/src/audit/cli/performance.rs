use serde_json::Value;
use std::path::Path;

const LAW: &str = "cli-performance-latency-speed-iteration-fitness";
const SCHEMA: &str = "schemas/cli-performance-receipt.schema.json";
const RECEIPT: &str = "validation_artifacts/cli/performance-receipt.json";
const SRC: &str = "validator/src/cli/performance.rs";
const TYPES: &str = "validator/src/cli/performance/types.rs";
const REDS: &[&str] = &[
    "cli-performance-missing-budget-red",
    "cli-performance-prose-only-budget-red",
    "cli-performance-missing-receipt-red",
    "cli-performance-stale-receipt-red",
    "cli-performance-wrong-candidate-receipt-red",
    "cli-performance-wrong-cli-binary-receipt-red",
    "cli-performance-wrong-schema-law-fixture-digest-red",
    "cli-performance-over-budget-without-claim-blocking-red",
    "cli-performance-focused-substituted-for-final-red",
    "cli-performance-cached-proof-substituted-for-no-cache-red",
    "cli-performance-hidden-cache-pass-red",
    "cli-performance-unbounded-concurrency-red",
    "cli-performance-serial-global-lock-red",
    "cli-performance-network-call-in-local-only-red",
    "cli-performance-missing-timeout-red",
    "cli-performance-missing-live-probe-backoff-red",
    "cli-performance-missing-external-call-telemetry-red",
    "cli-performance-missing-input-size-telemetry-red",
    "cli-performance-missing-fixture-count-telemetry-red",
    "cli-performance-missing-cache-key-telemetry-red",
    "cli-performance-missing-regression-baseline-red",
    "cli-performance-final-packet-omits-status-red",
];

pub fn package_failures(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    require_files(root, &mut out);
    require_inventory(root, &mut out);
    require_catalog(root, &mut out);
    require_law_rows(root, &mut out);
    require_reds(root, &mut out);
    require_receipt(root, &mut out);
    out
}

fn require_files(root: &Path, out: &mut Vec<String>) {
    for rel in [SRC, TYPES, SCHEMA] {
        if !root.join(rel).is_file() {
            out.push(format!("cli_performance_missing_artifact:{rel}"));
        }
    }
}

fn require_inventory(root: &Path, out: &mut Vec<String>) {
    let manifest = read_json(root, "plugin-manifest-draft.json");
    let inventory = crate::package::inventory::inventory_paths(&manifest);
    for rel in [SRC, TYPES, SCHEMA, RECEIPT] {
        if !inventory.iter().any(|path| path == rel) {
            out.push(format!("cli_performance_package_inventory_missing:{rel}"));
        }
    }
}

fn require_catalog(root: &Path, out: &mut Vec<String>) {
    let catalog = read_json(root, "schemas/schema-catalog.json");
    let found = catalog
        .get("schemas")
        .and_then(Value::as_array)
        .is_some_and(|rows| {
            rows.iter()
                .any(|row| row.get("path").and_then(Value::as_str) == Some(SCHEMA))
        });
    if !found {
        out.push("cli_performance_schema_catalog_missing_receipt_schema".to_string());
    }
}

fn require_law_rows(root: &Path, out: &mut Vec<String>) {
    for (rel, key, code) in [
        (
            "templates/agent-standards/enforcement.json",
            "rows",
            "missing_standards_row",
        ),
        (
            "docs/source-obligation-matrix.json",
            "obligations",
            "missing_source_obligation",
        ),
        (
            "docs/foundational-law-traceability.json",
            "entries",
            "missing_foundational_trace",
        ),
    ] {
        if !has_law_id(root, rel, key) {
            out.push(format!("cli_performance_{code}:{LAW}"));
        }
    }
    if !root
        .join("fixtures/mandatory-law-surfaces/valid")
        .join(format!("{LAW}.json"))
        .is_file()
    {
        out.push(format!("cli_performance_missing_valid_fixture:{LAW}"));
    }
}

fn require_reds(root: &Path, out: &mut Vec<String>) {
    let catalog = read_json(root, "templates/RED_FIXTURES.json");
    for red in REDS {
        if !array_contains_id(&catalog, red) {
            out.push(format!("cli_performance_missing_red_fixture:{red}"));
        }
    }
}

fn require_receipt(root: &Path, out: &mut Vec<String>) {
    match crate::json_boundary::read_json(&root.join(RECEIPT)) {
        Ok(value) => out.extend(crate::cli::performance::receipt::surface_value_failures(
            &value,
        )),
        Err(_) => out.push(format!(
            "cli_performance_missing_fail_closed_receipt:{RECEIPT}"
        )),
    }
}

fn has_law_id(root: &Path, rel: &str, key: &str) -> bool {
    read_json(root, rel)
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(|rows| {
            rows.iter().any(|row| {
                row.get("id")
                    .or_else(|| row.get("obligation_id"))
                    .and_then(Value::as_str)
                    == Some(LAW)
            })
        })
}

fn array_contains_id(value: &Value, id: &str) -> bool {
    value.as_array().is_some_and(|rows| {
        rows.iter()
            .any(|row| row.get("id").and_then(Value::as_str) == Some(id))
    })
}

fn read_json(root: &Path, rel: &str) -> Value {
    crate::json_boundary::read_json(&root.join(rel)).unwrap_or(Value::Null)
}
