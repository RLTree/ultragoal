use serde_json::Value;
use std::path::Path;

pub(super) const REQUIRED_FILES: &[&str] = &[
    "dev/observability/compose.yml",
    "dev/observability/otel-collector/config.yaml",
    "dev/observability/vector/vector.yaml",
    "dev/observability/grafana/provisioning/datasources/datasources.yml",
    "docs/generated/observability/command-inventory.json",
    "schemas/observability-event.schema.json",
    "schemas/observability-metric.schema.json",
    "schemas/observability-trace.schema.json",
    "schemas/observability-receipt.schema.json",
    "schemas/observability-query-result.schema.json",
    "validator/src/cli/observe/mod.rs",
    "validator/src/cli/observe/types.rs",
    "validator/src/cli/observe/telemetry/mod.rs",
    "validator/src/cli/observe/stack/mod.rs",
    "validator/src/cli/observe/query/mod.rs",
    "validator/src/cli/observe/explain/mod.rs",
];

pub(super) const REQUIRED_SCHEMA_PATHS: &[&str] = &[
    "schemas/observability-event.schema.json",
    "schemas/observability-metric.schema.json",
    "schemas/observability-trace.schema.json",
    "schemas/observability-receipt.schema.json",
    "schemas/observability-query-result.schema.json",
];

pub(super) fn check(root: &Path, out: &mut Vec<String>) {
    require_files(root, out);
    require_inventory(root, out);
    require_schema_catalog(root, out);
}

fn require_files(root: &Path, out: &mut Vec<String>) {
    for rel in REQUIRED_FILES {
        if !root.join(rel).is_file() {
            out.push(format!("observability_missing_artifact:{rel}"));
        }
    }
}

fn require_inventory(root: &Path, out: &mut Vec<String>) {
    let manifest = super::read::json(root, "plugin-manifest-draft.json");
    let inventory = crate::package::inventory::inventory_paths(&manifest);
    for rel in REQUIRED_FILES.iter().chain(REQUIRED_SCHEMA_PATHS.iter()) {
        if !inventory.iter().any(|path| path == rel) {
            out.push(format!("observability_package_inventory_missing:{rel}"));
        }
    }
}

fn require_schema_catalog(root: &Path, out: &mut Vec<String>) {
    let catalog = super::read::json(root, "schemas/schema-catalog.json");
    for schema in REQUIRED_SCHEMA_PATHS {
        if !catalog_has_path(&catalog, schema) {
            out.push(format!("observability_schema_catalog_missing:{schema}"));
        }
    }
}

fn catalog_has_path(value: &Value, path: &str) -> bool {
    value
        .get("schemas")
        .and_then(Value::as_array)
        .is_some_and(|rows| {
            rows.iter()
                .any(|row| row.get("path").and_then(Value::as_str) == Some(path))
        })
}
