use serde_json::Value;
use std::path::Path;

pub(crate) const LAWS: &[RustLaw] = &[
    RustLaw {
        id: "rust-developer-experience-authority",
        receipt: "validation_artifacts/rust/dependency-receipt.json",
        red: "rust-devx-raw-tool-output-substitution-red",
    },
    RustLaw {
        id: "rust-toolchain-substrate-authority",
        receipt: "validation_artifacts/rust/toolchain-receipt.json",
        red: "rust-devx-missing-toolchain-substrate-red",
    },
    RustLaw {
        id: "rust-command-loop-authority",
        receipt: "validation_artifacts/rust/standard-receipt.json",
        red: "rust-devx-cargo-test-overclaim-red",
    },
    RustLaw {
        id: "rust-cache-no-cache-honesty",
        receipt: "validation_artifacts/rust/clean-proof-receipt.json",
        red: "rust-devx-hidden-cache-no-cache-substitution-red",
    },
    RustLaw {
        id: "rust-memory-resource-discipline",
        receipt: "validation_artifacts/rust/memory-receipt.json",
        red: "rust-devx-unbounded-resource-discipline-red",
    },
    RustLaw {
        id: "workspace-artifact-cache-garbage-collection",
        receipt: "validation_artifacts/gc/verify-receipt.json",
        red: "rust-devx-blind-cleanup-without-gc-receipt-red",
    },
];

const RUST_SCHEMA: &str = "schemas/rust-devx-receipt.schema.json";
const GC_SCHEMA: &str = "schemas/workspace-gc-receipt.schema.json";
const RUST_SRC: &[&str] = &[
    "validator/src/cli/rust/mod.rs",
    "validator/src/cli/rust/observations.rs",
    "validator/src/cli/rust/types.rs",
    "validator/src/cli/rust/receipt.rs",
    "validator/src/audit/rust/developer.rs",
];
const GC_SRC: &[&str] = &[
    "validator/src/cli/garbage/collection/mod.rs",
    "validator/src/cli/garbage/collection/types.rs",
    "validator/src/cli/garbage/collection/receipt.rs",
];

pub(crate) struct RustLaw {
    pub(crate) id: &'static str,
    pub(crate) receipt: &'static str,
    pub(crate) red: &'static str,
}

pub(crate) fn package_failures(root: &Path) -> Vec<(String, String)> {
    let candidate = crate::package::inventory::package_digest(root).unwrap_or_default();
    LAWS.iter()
        .flat_map(|law| {
            law_failures(root, law, &candidate)
                .into_iter()
                .map(|failure| (law.id.to_string(), failure))
        })
        .collect()
}

fn law_failures(root: &Path, law: &RustLaw, candidate: &str) -> Vec<String> {
    let mut out = Vec::new();
    require_files(root, law, &mut out);
    require_inventory(root, law, &mut out);
    require_catalog(root, law, &mut out);
    require_law_rows(root, law, &mut out);
    require_red(root, law, &mut out);
    require_receipt(root, law, candidate, &mut out);
    out
}

fn require_files(root: &Path, law: &RustLaw, out: &mut Vec<String>) {
    for rel in required_files(law) {
        if root.join(rel).is_file() {
            continue;
        }
        out.push(format!("rust_devx_missing_artifact:{rel}"));
    }
}

fn require_inventory(root: &Path, law: &RustLaw, out: &mut Vec<String>) {
    let manifest = read_json(root, "plugin-manifest-draft.json");
    let inventory = crate::package::inventory::inventory_paths(&manifest);
    for rel in required_files(law) {
        if inventory.iter().any(|path| path == rel) {
            continue;
        }
        out.push(format!("rust_devx_package_inventory_missing:{rel}"));
    }
}

fn require_catalog(root: &Path, law: &RustLaw, out: &mut Vec<String>) {
    let catalog = read_json(root, "schemas/schema-catalog.json");
    for schema in required_schemas(law) {
        let found = catalog
            .get("schemas")
            .and_then(Value::as_array)
            .is_some_and(|rows| {
                rows.iter()
                    .any(|row| row.get("path").and_then(Value::as_str) == Some(schema))
            });
        if found {
            continue;
        }
        out.push(format!("rust_devx_schema_catalog_missing:{schema}"));
    }
}

fn require_law_rows(root: &Path, law: &RustLaw, out: &mut Vec<String>) {
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
        if has_law_id(root, rel, key, law.id) {
            continue;
        }
        out.push(format!("rust_devx_{code}:{}", law.id));
    }
    let valid = format!("fixtures/mandatory-law-surfaces/valid/{}.json", law.id);
    if root.join(&valid).is_file() {
        return;
    }
    out.push(format!("rust_devx_missing_valid_fixture:{valid}"));
}

fn require_red(root: &Path, law: &RustLaw, out: &mut Vec<String>) {
    let catalog = read_json(root, "templates/RED_FIXTURES.json");
    let found = catalog.as_array().is_some_and(|rows| {
        rows.iter()
            .any(|row| row.get("id").and_then(Value::as_str) == Some(law.red))
    });
    if found {
        return;
    }
    out.push(format!("rust_devx_missing_red_fixture:{}", law.red));
}

pub(crate) fn require_receipt(root: &Path, law: &RustLaw, candidate: &str, out: &mut Vec<String>) {
    match crate::json_boundary::read_json(&root.join(law.receipt)) {
        Ok(value) => out.extend(receipt_failures(&value, law, candidate)),
        Err(_) => out.push(format!("rust_devx_missing_receipt:{}", law.receipt)),
    }
}

pub(crate) fn receipt_failures(value: &Value, law: &RustLaw, candidate: &str) -> Vec<String> {
    if law.id == "workspace-artifact-cache-garbage-collection" {
        let mut out = crate::cli::garbage::collection::receipt::surface_value_failures(value);
        if value.pointer("/digests/candidate").and_then(Value::as_str) != Some(candidate) {
            out.push("workspace_gc_receipt_candidate_digest_mismatch".to_string());
        }
        return out;
    }
    let mut out = crate::cli::rust::receipt::surface_value_failures(value, law.id);
    if value.pointer("/digests/candidate").and_then(Value::as_str) != Some(candidate) {
        out.push("rust_devx_receipt_candidate_digest_mismatch".to_string());
    }
    out
}

fn required_files(law: &RustLaw) -> Vec<&'static str> {
    if law.id == "workspace-artifact-cache-garbage-collection" {
        GC_SRC.iter().copied().chain([GC_SCHEMA]).collect()
    } else {
        RUST_SRC.iter().copied().chain([RUST_SCHEMA]).collect()
    }
}

pub(crate) fn required_schemas(law: &RustLaw) -> Vec<&'static str> {
    if law.id == "workspace-artifact-cache-garbage-collection" {
        vec![GC_SCHEMA]
    } else {
        vec![RUST_SCHEMA]
    }
}

pub(crate) fn has_law_id(root: &Path, rel: &str, key: &str, id: &str) -> bool {
    read_json(root, rel)
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(|rows| {
            rows.iter().any(|row| {
                row.get("id")
                    .or_else(|| row.get("obligation_id"))
                    .and_then(Value::as_str)
                    == Some(id)
            })
        })
}

fn read_json(root: &Path, rel: &str) -> Value {
    crate::json_boundary::read_json(&root.join(rel)).unwrap_or(Value::Null)
}
