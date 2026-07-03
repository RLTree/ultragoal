use super::{HaloCommand, proof};
use command_fixtures::*;
use serde_json::json;
use std::path::{Path, PathBuf};

mod command_fixtures;

#[test]
fn halo_desktop_capability_is_manual_only() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("halo-capability");
    seed_root(&root);
    let app = fake_app(&root, "0.1.17");
    let command = HaloCommand {
        receipt: PathBuf::from(super::DEFAULT_RECEIPT),
        app_path: app,
    };
    let receipt = proof::build_receipt(&root, &command).expect("receipt");
    assert_eq!(receipt["status"], "pass");
    assert_eq!(receipt["invocation_mode"], "desktop_manual");
    assert_eq!(receipt["authority_class"], "manual_observation_only");
    assert!(proof::receipt_failures(&root, &receipt).is_empty());
}

#[test]
fn halo_capability_rejects_overbroad_authority() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("halo-overbroad");
    seed_root(&root);
    let command = HaloCommand {
        receipt: PathBuf::from(super::DEFAULT_RECEIPT),
        app_path: fake_app(&root, "0.1.17"),
    };
    let mut receipt = proof::build_receipt(&root, &command).expect("receipt");
    receipt["authority_class"] = json!("adapter_required_before_claims");
    assert!(
        proof::receipt_failures(&root, &receipt)
            .iter()
            .any(|item| item == "halo_capability_authority_overbroad")
    );
}

#[test]
fn halo_run_parse_detection_and_registry_edges_are_typed() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("halo-run");
    seed_root(&root);
    let app = fake_app(&root, "0.1.18");
    let receipt_path = root.join("validation_artifacts/halo/run-receipt.json");
    let raw = [
        "halo",
        "capability",
        "prove",
        "--app-path",
        app.to_str().expect("app path"),
        "--receipt",
        receipt_path.to_str().expect("receipt path"),
    ]
    .into_iter()
    .map(ToString::to_string)
    .collect::<Vec<_>>();
    assert!(
        super::parse(&["not-halo".to_string()])
            .expect("not halo")
            .is_none()
    );
    assert!(super::parse(&["halo".to_string()]).is_err());
    let command = super::parse(&raw)
        .expect("parse halo")
        .expect("halo command");
    assert_eq!(super::run(&root, &command).expect("run halo"), 0);
    let receipt = crate::json_boundary::read_json(&receipt_path).expect("receipt");
    assert!(super::receipt_failures(&root, &receipt).is_empty());
    let obs = receipt
        .get("observability_receipt")
        .and_then(serde_json::Value::as_object)
        .expect("observability receipt");
    for key in [
        "run_id",
        "correlation_id",
        "trace_id",
        "failure_class",
        "claim_impact",
    ] {
        assert!(
            obs.get(key)
                .and_then(serde_json::Value::as_str)
                .is_some_and(|value| !value.is_empty()),
            "{key}: {receipt}"
        );
    }
    assert!(
        obs.get("trace")
            .and_then(serde_json::Value::as_object)
            .and_then(|trace| trace.get("span_id"))
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| !value.is_empty()),
        "trace span_id: {receipt}"
    );

    let missing_app = proof::build_receipt(
        &root,
        &HaloCommand {
            receipt: PathBuf::from(super::DEFAULT_RECEIPT),
            app_path: root.join("Missing.app"),
        },
    )
    .expect("missing app");
    assert_eq!(missing_app["invocation_mode"], "unavailable");
    assert_eq!(missing_app["authority_class"], "fail_closed");
    assert_eq!(missing_app["bundle_observed"], false);

    let configured = super::detect::app(&app);
    assert_eq!(configured.path_class, "configured_local_path");
    assert_eq!(configured.identifier, "net.inference.halo");
    assert_eq!(configured.version, "0.1.18");
    let system = super::detect::app(Path::new("/Applications/HALO-does-not-exist.app"));
    assert_eq!(system.path_class, "system_applications");
    assert_eq!(system.identifier, "unknown");
    assert!(!super::detect::command_on_paths(
        "halo",
        None::<std::ffi::OsString>
    ));
    assert!(!super::detect::command_on_path(
        "ultragoal-halo-command-that-does-not-exist"
    ));

    let relative = HaloCommand {
        receipt: PathBuf::from("validation_artifacts/halo/relative-receipt.json"),
        app_path: fake_app(&root, "0.1.19"),
    };
    assert_eq!(super::run(&root, &relative).expect("relative halo run"), 0);
    assert!(
        root.join("validation_artifacts/halo/relative-receipt.json")
            .is_file()
    );

    let bad_root = root_without_registry();
    let bad_registry = proof::build_receipt(
        &bad_root,
        &HaloCommand {
            receipt: PathBuf::from(super::DEFAULT_RECEIPT),
            app_path: app,
        },
    )
    .expect("bad registry");
    assert_eq!(bad_registry["status"], "fail");
    assert!(
        bad_registry["failures"]
            .as_array()
            .expect("failures")
            .iter()
            .any(|failure| failure == "halo_adapter_registry_wrong_schema")
    );
    std::fs::remove_dir_all(bad_root).expect("cleanup bad registry");

    let secret_app = fake_app_with_identifier(&root, "Authorization: Bearer redacted", "0.1.18");
    let secret = proof::build_receipt(
        &root,
        &HaloCommand {
            receipt: PathBuf::from(super::DEFAULT_RECEIPT),
            app_path: secret_app,
        },
    )
    .expect("secret halo");
    assert_eq!(secret["status"], "fail");
    assert_eq!(
        secret["failures"][0],
        "halo_capability_receipt_secret_shape_detected"
    );

    let failures = super::receipt_failures(&root, &json!({}));
    for expected in [
        "halo_capability_receipt_field_mismatch:schema",
        "halo_capability_receipt_field_mismatch:status",
        "halo_capability_receipt_field_mismatch:candidate_digest",
        "halo_capability_receipt_field_mismatch:raw_halo_authority",
        "halo_capability_receipt_missing_claim_blockers",
        "halo_capability_observability_binding_invalid",
    ] {
        assert!(
            failures.contains(&expected.to_string()),
            "{expected}: {failures:?}"
        );
    }
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn halo_defaults_and_run_error_boundaries_are_typed() {
    let default = super::parse(&[
        "halo".to_string(),
        "capability".to_string(),
        "prove".to_string(),
    ])
    .expect("parse default")
    .expect("halo default");
    assert_eq!(default.receipt, PathBuf::from(super::DEFAULT_RECEIPT));
    assert_eq!(default.app_path, PathBuf::from(super::DEFAULT_APP));
    super::print_receipt(Path::new("receipt.json"), &json!({}));

    let missing_root = std::env::temp_dir().join(format!(
        "ultragoal-halo-missing-root-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&missing_root);
    assert!(super::run(&missing_root, &default).is_err());

    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("halo-write-error");
    seed_root(&root);
    std::fs::create_dir_all(root.join("validation_artifacts")).expect("artifacts");
    std::fs::write(root.join("validation_artifacts/halo"), "not a directory").expect("blocker");
    let command = HaloCommand {
        receipt: PathBuf::from("validation_artifacts/halo/write-error.json"),
        app_path: root.join("Missing.app"),
    };
    assert!(super::run(&root, &command).is_err());
    std::fs::remove_dir_all(root).expect("cleanup write error");
}
