use crate::cli::control::plane::surface;
use crate::cli::control::plane::types::ControlOperation;
use serde_json::json;

#[test]
fn package_surface_validation_reports_malformed_and_weak_blockers() {
    let candidate = crate::self_tests::boundaries::support::sha('a');
    let mut malformed = json!({
        "schema": "wrong",
        "operation": "cache_audit",
        "candidate_digest": crate::self_tests::boundaries::support::sha('b'),
        "source": {"package_digest": crate::self_tests::boundaries::support::sha('c')},
        "target": {"surface": "versioned_cache_package", "local_path": "redacted-local-proof-path"},
        "status": "unknown",
        "claim_ceiling": "surface_package_digest_aligned",
        "same_candidate": true,
        "failures": []
    });
    let failures = surface::same_candidate_pass_or_fail_closed_failures(
        &malformed,
        &candidate,
        ControlOperation::InstallAudit,
    );
    assert_surface_validation_failures(&failures);

    malformed["status"] = json!("pass");
    malformed["claim_ceiling"] = json!("withheld_or_blocked");
    let failures = surface::same_candidate_pass_or_fail_closed_failures(
        &malformed,
        &candidate,
        ControlOperation::InstallAudit,
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure == "package_surface_audit_claim_ceiling_not_surface_aligned")
    );
}

#[test]
fn package_cache_surface_validation_reports_malformed_and_weak_blockers() {
    let candidate = crate::self_tests::boundaries::support::sha('a');
    let mut malformed = json!({
        "schema": "wrong",
        "operation": "install_audit",
        "candidate_digest": crate::self_tests::boundaries::support::sha('b'),
        "source": {"package_digest": crate::self_tests::boundaries::support::sha('c')},
        "target": {"surface": "installed_plugin", "local_path": "redacted-local-proof-path"},
        "status": "unknown",
        "claim_ceiling": "surface_package_digest_aligned",
        "same_candidate": true,
        "failures": []
    });
    let failures = surface::same_candidate_pass_or_fail_closed_failures(
        &malformed,
        &candidate,
        ControlOperation::CacheAudit,
    );
    assert_surface_validation_failures(&failures);

    malformed["status"] = json!("pass");
    malformed["claim_ceiling"] = json!("withheld_or_blocked");
    let failures = surface::same_candidate_pass_or_fail_closed_failures(
        &malformed,
        &candidate,
        ControlOperation::CacheAudit,
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure == "package_surface_audit_claim_ceiling_not_surface_aligned")
    );
}

fn assert_surface_validation_failures(failures: &[String]) {
    for expected in [
        "package_surface_audit_wrong_schema",
        "package_surface_audit_wrong_operation",
        "package_surface_audit_candidate_digest_mismatch",
        "package_surface_audit_source_digest_mismatch",
        "package_surface_audit_target_expected_digest_mismatch",
        "package_surface_audit_target_surface_mismatch",
        "package_surface_audit_private_local_path_present",
        "package_surface_audit_status_not_pass_or_fail",
        "package_surface_audit_fail_closed_claim_ceiling_not_blocking",
        "package_surface_audit_fail_closed_same_candidate_not_false",
        "package_surface_audit_fail_closed_missing_failures",
    ] {
        assert!(
            failures.iter().any(|failure| failure == expected),
            "{expected}: {failures:?}"
        );
    }
}
