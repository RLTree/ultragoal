use crate::cli::control::plane::{proof, types::ControlOperation};
use serde_json::json;

#[test]
fn update_goal_requires_transactional_finalization_receipt() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("cli-transaction-missing");
    super::write_manifest(&root);
    let observed = proof::failures(&root, ControlOperation::UpdateGoalEligibility);
    assert!(
        observed
            .iter()
            .any(|failure| failure
                .starts_with("cli_control_plane_transactional_finalization_missing:")),
        "{observed:?}"
    );
    let registry_probe = proof::failures(&root, ControlOperation::RegistryProbe);
    assert!(
        !registry_probe
            .iter()
            .any(|failure| failure.starts_with("cli_control_plane_transaction")),
        "{registry_probe:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup transaction missing");
}

#[test]
fn transaction_reports_digest_unavailable_before_receipt_theater() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("cli-transaction-no-digest");
    super::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["missing.txt"]}),
    );
    let failures = proof::failures(&root, ControlOperation::UpdateGoalEligibility);
    assert!(
        failures
            .iter()
            .any(|failure| failure.starts_with("cli_control_plane_transaction_digest_unavailable")),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup transaction no digest");
}

#[test]
fn transaction_finalize_reports_observability_spool_write_failures() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "transaction-command-spool-file",
    );
    super::write_manifest(&root);
    let spool_path = root.join("validation_artifacts/observability/spool");
    std::fs::create_dir_all(spool_path.parent().expect("spool parent")).expect("spool parent");
    std::fs::write(&spool_path, "not a directory").expect("spool file");
    let err = crate::command_run::run_with_exit_code(crate::Args {
        root: root.clone(),
        command: crate::Command::TransactionalFinalization {
            receipt: std::path::PathBuf::from(super::RECEIPT),
        },
    })
    .expect_err("spool file blocks telemetry emission");

    assert!(
        err.contains("validation_artifacts/observability/spool"),
        "{err}"
    );
    std::fs::remove_dir_all(root).expect("cleanup transaction spool file");
}

#[test]
fn transaction_finalize_rejects_ungoverned_claim_receipt_outputs() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "transaction-command-ungoverned-receipt",
    );
    super::write_manifest(&root);

    let err = crate::command_run::run_with_exit_code(crate::Args {
        root: root.clone(),
        command: crate::Command::TransactionalFinalization {
            receipt: std::path::PathBuf::from("target/transaction-finalization.json"),
        },
    })
    .expect_err("ungoverned transaction receipt path rejected");

    assert!(err.contains("transactional finalization receipt"), "{err}");
    assert!(err.contains("governed claim artifact root"), "{err}");
    assert!(err.contains("external debug only"), "{err}");
    assert!(!root.join("target/transaction-finalization.json").exists());
    std::fs::remove_dir_all(root).expect("cleanup transaction ungoverned receipt");
}
