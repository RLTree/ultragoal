use serde_json::{Value, json};
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("json write");
}

fn minimal_root(label: &str) -> std::path::PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({
            "name": "typed-boundaries-artifact-test",
            "version": "0.0.0",
            "resources": []
        }),
    );
    root
}

#[test]
fn typed_boundaries_receipt_uses_bounded_inventory_summaries() {
    let root = minimal_root("typed-boundaries-artifact-contract");
    let receipt = std::path::PathBuf::from("validation_artifacts/typed-boundaries/artifacts.json");
    let exit = super::super::run(
        &root,
        &super::super::TypedBoundariesCommand {
            receipt: receipt.clone(),
            jobs: Some(1),
        },
    )
    .expect("typed-boundaries run");
    assert_eq!(exit, 1);

    let receipt_path =
        crate::output_path::claim_artifact_path(&root, &receipt, "test typed boundaries receipt")
            .expect("typed receipt path");
    let value = crate::json_boundary::read_json(&receipt_path).expect("receipt");
    assert_inventory_summary(&root, &value, "foundational_law_surface_inventory");
    assert_inventory_summary(
        &root,
        &value["foundational_law_surface_inventory"],
        "package_surface_inventory",
    );
    assert!(
        !super::super::PACKAGE_SURFACE_INVENTORY_REL
            .starts_with("validation_artifacts/observability/"),
        "full package surface rows must not live under the observability redaction scan root"
    );
    let second_exit = super::super::run(
        &root,
        &super::super::TypedBoundariesCommand {
            receipt: receipt.clone(),
            jobs: Some(1),
        },
    )
    .expect("second typed-boundaries run");
    assert_eq!(second_exit, 1);
    let third_exit = super::super::run(
        &root,
        &super::super::TypedBoundariesCommand {
            receipt: receipt.clone(),
            jobs: Some(1),
        },
    )
    .expect("third typed-boundaries run");
    assert_eq!(third_exit, 1);
    let third_value = crate::json_boundary::read_json(&receipt_path).expect("third receipt");
    assert_eq!(
        third_value["foundational_law_surface_inventory"]["row_materialization"]["artifact_status"],
        "reused_current_artifact"
    );
    assert_eq!(
        third_value["foundational_law_surface_inventory"]["package_surface_inventory"]["row_materialization"]
            ["artifact_status"],
        "reused_current_artifact"
    );
    assert!(
        value["event"]["foundational_law_surface_inventory"]["package_surface_inventory"]
            .get("rows")
            .is_none(),
        "{}",
        value["event"]["foundational_law_surface_inventory"]["package_surface_inventory"]
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn typed_boundaries_receipt_cannot_overwrite_inventory_artifacts() {
    let root = minimal_root("typed-boundaries-artifact-collision");
    let err = super::super::run(
        &root,
        &super::super::TypedBoundariesCommand {
            receipt: std::path::PathBuf::from(super::super::PACKAGE_SURFACE_INVENTORY_REL),
            jobs: Some(1),
        },
    )
    .expect_err("reserved package inventory artifact path rejected");
    assert!(err.contains("reserved inventory artifact"), "{err}");
    assert!(
        !root
            .join(super::super::PACKAGE_SURFACE_INVENTORY_REL)
            .exists(),
        "collision must fail before writing the artifact"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn typed_boundaries_reports_foundational_inventory_artifact_write_failure() {
    let root = minimal_root("typed-boundaries-foundational-artifact-write-fail");
    std::fs::create_dir_all(root.join(super::super::FOUNDATIONAL_SURFACE_INVENTORY_REL))
        .expect("artifact directory");
    let err = super::super::run(
        &root,
        &super::super::TypedBoundariesCommand {
            receipt: std::path::PathBuf::from(
                "validation_artifacts/typed-boundaries/typed-boundaries.json",
            ),
            jobs: Some(1),
        },
    )
    .expect_err("foundational inventory artifact write failure");
    assert!(
        err.contains("foundational surface inventory artifact"),
        "{err}"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn typed_boundaries_reports_package_inventory_artifact_write_failure() {
    let root = minimal_root("typed-boundaries-package-artifact-write-fail");
    std::fs::create_dir_all(root.join(super::super::PACKAGE_SURFACE_INVENTORY_REL))
        .expect("artifact directory");
    let err = super::super::run(
        &root,
        &super::super::TypedBoundariesCommand {
            receipt: std::path::PathBuf::from(
                "validation_artifacts/typed-boundaries/typed-boundaries.json",
            ),
            jobs: Some(1),
        },
    )
    .expect_err("package inventory artifact write failure");
    assert!(err.contains("package surface inventory artifact"), "{err}");
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn typed_boundaries_reports_receipt_write_failure_after_artifacts_close() {
    let root = minimal_root("typed-boundaries-receipt-write-fail");
    let receipt = std::path::PathBuf::from("validation_artifacts/typed-boundaries/receipt.json");
    std::fs::create_dir_all(root.join(&receipt)).expect("receipt path directory");
    let err = super::super::run(
        &root,
        &super::super::TypedBoundariesCommand {
            receipt,
            jobs: Some(1),
        },
    )
    .expect_err("receipt write failure");
    assert!(err.contains("json rename failed"), "{err}");
    assert!(
        root.join(super::super::FOUNDATIONAL_SURFACE_INVENTORY_REL)
            .is_file(),
        "inventory artifacts should close before final receipt write failure"
    );
    assert!(
        root.join(super::super::PACKAGE_SURFACE_INVENTORY_REL)
            .is_file(),
        "package inventory artifact should close before final receipt write failure"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

fn assert_inventory_summary(root: &Path, value: &Value, field: &str) {
    let summary = &value[field];
    assert_eq!(summary["rows_omitted_from_receipt"], true);
    assert!(summary.get("rows").is_none(), "{summary}");
    let event_summary = if field == "foundational_law_surface_inventory" {
        &value["event"][field]
    } else {
        &Value::Null
    };
    if !event_summary.is_null() {
        assert!(event_summary.get("rows").is_none(), "{event_summary}");
    }
    let artifact_rel = summary["row_materialization"]["artifact_path"]
        .as_str()
        .expect("artifact path");
    let artifact_path = root.join(artifact_rel);
    assert!(artifact_path.is_file(), "missing {artifact_rel}");
    assert_eq!(
        summary["row_materialization"]["artifact_digest"]
            .as_str()
            .expect("artifact digest field"),
        crate::digest::file(&artifact_path).expect("artifact digest")
    );
    let artifact = crate::json_boundary::read_json(&artifact_path).expect("surface artifact");
    let rows = artifact
        .get("rows")
        .and_then(serde_json::Value::as_array)
        .expect("materialized rows");
    assert_eq!(
        summary["rows_digest"].as_str().expect("summary row digest"),
        crate::digest::canonical_json(&Value::Array(rows.clone()))
    );
    assert!(
        rows.iter()
            .filter_map(|row| row.get("path").and_then(Value::as_str))
            .all(|path| path != artifact_rel),
        "materialized inventory artifact must not scan itself as runtime evidence"
    );
}
