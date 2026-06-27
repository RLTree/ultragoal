use crate::cli::control::plane::{proof, types::ControlOperation};
use serde_json::json;

#[test]
fn update_goal_requires_transactional_finalization_receipt() {
    let root = crate::self_tests::boundaries::support::temp_root("cli-transaction-missing");
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
    let root = crate::self_tests::boundaries::support::temp_root("cli-transaction-no-digest");
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
