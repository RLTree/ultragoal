use crate::cli::control::plane::path::{expected_receipt_path, validate_receipt_path};
use crate::cli::control::plane::types::ControlOperation;
use crate::cli::control::plane::{ControlCommand, run};
use serde_json::json;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("validator has repo parent")
        .to_path_buf()
}

fn write_json(path: &Path, value: &serde_json::Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

#[test]
fn control_receipt_paths_are_operation_owned_cli_surfaces() {
    let root = repo_root();
    assert_eq!(
        expected_receipt_path(ControlOperation::UpdateGoalEligibility),
        "validation_artifacts/cli/update-goal-eligibility.json"
    );
    assert_eq!(
        expected_receipt_path(ControlOperation::SelfUpdateGoalEligibility),
        "validation_artifacts/cli/self-law-receipt.json"
    );
    assert_eq!(
        expected_receipt_path(ControlOperation::FixturesAll),
        "validation_artifacts/cli/fixtures-all-receipt.json"
    );
    assert!(
        validate_receipt_path(
            &root,
            Path::new("validation_artifacts/cli/registry-probe-receipt.json"),
            ControlOperation::RegistryProbe,
        )
        .is_ok()
    );
    assert!(
        validate_receipt_path(
            &root,
            Path::new("./validation_artifacts/cli/registry-probe-receipt.json"),
            ControlOperation::RegistryProbe,
        )
        .is_ok()
    );
    assert!(
        validate_receipt_path(
            &root,
            &root.join("validation_artifacts/cli/packet-verify-receipt.json"),
            ControlOperation::PacketVerify,
        )
        .is_ok()
    );
}

#[test]
fn control_receipt_paths_reject_wrong_schema_proof_surfaces() {
    let root = repo_root();
    for (operation, bad_path) in [
        (
            ControlOperation::RegistryProbe,
            "validation_artifacts/ultragoal-audit/active-registry-exposure-current.json",
        ),
        (
            ControlOperation::PacketVerify,
            "validation_artifacts/review/final-packet-proof.json",
        ),
        (
            ControlOperation::UpdateGoalEligibility,
            "../validation_artifacts/cli/update-goal-eligibility.json",
        ),
    ] {
        let err = validate_receipt_path(&root, Path::new(bad_path), operation)
            .expect_err("bad path rejected");
        assert!(
            err.contains("cli_control_plane_receipt_path_"),
            "{operation:?}: {err}"
        );
    }
}

#[test]
fn control_receipt_paths_reject_outside_root_and_empty_paths() {
    let root = repo_root();
    for bad_path in [
        PathBuf::from("/tmp/harness-ultragoal-outside-receipt.json"),
        PathBuf::from(""),
    ] {
        let err = validate_receipt_path(&root, &bad_path, ControlOperation::RegistryProbe)
            .expect_err("bad path rejected");
        assert!(err.contains("cli_control_plane_receipt_path_"));
    }
}

#[test]
fn control_receipt_path_accepts_absolute_path_under_missing_root() {
    let root = repo_root().join("target/missing-cli-control-root");
    let receipt = root.join("validation_artifacts/cli/registry-probe-receipt.json");

    assert!(validate_receipt_path(&root, &receipt, ControlOperation::RegistryProbe).is_ok());
}

#[test]
fn run_refuses_law_specific_proof_paths_without_overwrite() {
    let root = crate::self_tests::boundaries::support::temp_root("cli-path-overwrite");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"version":"0.0.0-test","resources":[]}),
    );
    let registry_path =
        root.join("validation_artifacts/ultragoal-audit/active-registry-exposure-current.json");
    let packet_path = root.join("validation_artifacts/review/final-packet-proof.json");
    std::fs::create_dir_all(registry_path.parent().expect("registry parent")).expect("registry");
    std::fs::create_dir_all(packet_path.parent().expect("packet parent")).expect("packet");
    std::fs::write(&registry_path, b"registry-sentinel").expect("registry sentinel");
    std::fs::write(&packet_path, b"packet-sentinel").expect("packet sentinel");

    for (operation, path) in [
        (ControlOperation::RegistryProbe, registry_path.as_path()),
        (ControlOperation::PacketVerify, packet_path.as_path()),
    ] {
        let command = ControlCommand {
            operation,
            receipt: Some(path.to_path_buf()),
        };
        let err = run(&root, &command).expect_err("wrong surface rejected");
        assert!(
            err.contains("cli_control_plane_receipt_path_mismatch"),
            "{operation:?}: {err}"
        );
    }
    assert_eq!(
        std::fs::read_to_string(&registry_path).expect("registry read"),
        "registry-sentinel"
    );
    assert_eq!(
        std::fs::read_to_string(&packet_path).expect("packet read"),
        "packet-sentinel"
    );
    std::fs::remove_dir_all(root).expect("cleanup cli path overwrite");
}

#[test]
fn wrong_control_receipt_path_fails_before_package_digest() {
    let root = crate::self_tests::boundaries::support::temp_root("cli-path-before-digest");
    std::fs::create_dir_all(&root).expect("create cli path before digest root");
    let command = ControlCommand {
        operation: ControlOperation::RegistryProbe,
        receipt: Some(
            root.join("validation_artifacts/ultragoal-audit/active-registry-exposure-current.json"),
        ),
    };

    let err = run(&root, &command).expect_err("path rejected before package read");

    assert!(
        err.contains("cli_control_plane_receipt_path_mismatch"),
        "{err}"
    );
    assert!(!err.contains("plugin-manifest-draft"), "{err}");
    std::fs::remove_dir_all(root).expect("cleanup cli path before digest");
}
