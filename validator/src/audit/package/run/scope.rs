use std::collections::BTreeMap;

const DEFERRED_CHECK_IDS: &[&str] = &[
    "adversarial-packet-tampering-forged-proof-rejection",
    "cli-control-plane-authority",
    "cli-self-law-compliance",
    "generated-proof-artifact-provenance-anti-fabrication",
];

const DEFERRED_DETAIL_MARKERS: &[&str] = &[
    "app_registry",
    "cache-audit",
    "cache_audit",
    "cli_control_plane_receipt_",
    "final_packet_",
    "install-audit",
    "install_audit",
    "package_surface_audit_",
    "plugin_self_law_registry_",
    "registry_exposure",
    "reviewer_exposure",
    "transactional_finalization",
    "update-goal",
    "update_goal",
];

pub(crate) fn defer_out_of_scope_claims(mode: &str, failures: &mut BTreeMap<String, Vec<String>>) {
    if is_final_mode(mode) {
        return;
    }
    for check_id in DEFERRED_CHECK_IDS {
        if let Some(rows) = failures.get_mut(*check_id) {
            rows.clear();
        }
    }
    for rows in failures.values_mut() {
        rows.retain(|detail| !is_deferred_detail(detail));
    }
}

fn is_final_mode(mode: &str) -> bool {
    matches!(mode, "final" | "strict_final" | "strict-final")
}

fn is_deferred_detail(detail: &str) -> bool {
    DEFERRED_DETAIL_MARKERS
        .iter()
        .any(|marker| detail.contains(marker))
}

#[cfg(test)]
mod tests {
    use super::defer_out_of_scope_claims;
    use std::collections::BTreeMap;

    #[test]
    fn source_local_modes_defer_later_surfaces_without_hiding_local_failures() {
        let mut failures = BTreeMap::from([
            (
                "cli-control-plane-authority".to_string(),
                vec![
                    "validation_artifacts/cli/update-goal-eligibility.json: cli_control_plane_receipt_candidate_digest_mismatch".to_string(),
                    "mandatory_law_current_check_not_pass:cli-control-plane-authority:cli-control-plane-authority".to_string(),
                ],
            ),
            (
                "validator-execution-provenance".to_string(),
                vec![
                    "final_packet_proof_target_digest_mismatch".to_string(),
                    "session_log_hardening_package_digest_mismatch".to_string(),
                ],
            ),
            (
                "coverage-proof-accountability".to_string(),
                vec!["coverage_receipt_changed_files_digest_mismatch".to_string()],
            ),
        ]);
        defer_out_of_scope_claims("strict", &mut failures);
        assert!(failures["cli-control-plane-authority"].is_empty());
        assert_eq!(
            failures["validator-execution-provenance"],
            vec!["session_log_hardening_package_digest_mismatch"]
        );
        assert_eq!(
            failures["coverage-proof-accountability"],
            vec!["coverage_receipt_changed_files_digest_mismatch"]
        );
    }

    #[test]
    fn strict_final_keeps_later_surface_failures_blocking() {
        let mut failures = BTreeMap::from([(
            "cli-control-plane-authority".to_string(),
            vec!["cli_control_plane_receipt_candidate_digest_mismatch".to_string()],
        )]);
        defer_out_of_scope_claims("strict_final", &mut failures);
        assert_eq!(failures["cli-control-plane-authority"].len(), 1);
    }
}
