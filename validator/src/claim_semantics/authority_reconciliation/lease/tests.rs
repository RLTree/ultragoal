use super::debt;
use crate::audit::contract::Failure;
use serde_json::Value;
use std::fs;
use std::path::Path;

fn current_registry() -> Value {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    serde_json::from_str(&fs::read_to_string(root.join("LANE_REGISTRY.json")).expect("registry"))
        .expect("valid registry")
}

fn p0_mutation(mutate: impl FnOnce(&mut Value), expected: &str) {
    let registry = current_registry();
    let mut record = registry["lease_state"]["active_records"][0].clone();
    mutate(&mut record);
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let mut failures = Vec::<Failure>::new();
    debt::check(&record, &registry, root, &mut failures);
    assert!(
        failures.iter().any(|failure| failure.error == expected),
        "expected {expected}; got {failures:?}"
    );
}

#[test]
fn p0_rejects_missing_support_files() {
    p0_mutation(
        |record| {
            record
                .as_object_mut()
                .expect("record")
                .remove("support_files");
        },
        "p0_support_files_missing",
    );
}

#[test]
fn p0_rejects_support_diagnostic_overlap() {
    p0_mutation(
        |record| record["support_files"] = serde_json::json!([record["diagnostic_paths"][0]]),
        "p0_support_file_overlaps_diagnostic",
    );
}

#[test]
fn p0_rejects_normalized_support_alias_overlap() {
    p0_mutation(
        |record| {
            record["support_files"] = serde_json::json!([format!(
                "./{}",
                record["diagnostic_paths"][0].as_str().expect("path")
            )]);
        },
        "p0_support_file_overlaps_diagnostic",
    );
}

#[test]
fn p0_rejects_stale_diagnostic_digest() {
    p0_mutation(
        |record| {
            record["diagnostic_path_set_digest"] = serde_json::json!(
                "sha256:0000000000000000000000000000000000000000000000000000000000000000"
            )
        },
        "diagnostic_path_set_digest_mismatch",
    );
}

#[test]
fn p0_rejects_non_union_owned_files() {
    p0_mutation(
        |record| record["owned_files"] = record["diagnostic_paths"].clone(),
        "p0_dependency_closed_path_union_mismatch",
    );
}

#[test]
fn p0_rejects_extra_support_file() {
    p0_mutation(
        |record| {
            record["support_files"]
                .as_array_mut()
                .expect("support")
                .push(serde_json::json!(
                    "validator/src/claim_semantics/authority_reconciliation/lease.rs"
                ))
        },
        "p0_dependency_closed_path_union_mismatch",
    );
}
