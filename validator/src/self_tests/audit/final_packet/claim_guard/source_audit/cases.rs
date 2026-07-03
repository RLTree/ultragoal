use serde_json::{Value, json};
use std::path::Path;

pub(super) fn failure_cases(current: &str) -> Vec<(Value, &'static str)> {
    vec![
        (
            json!({"target_revision":{"kind":"package_digest","value":current},
                "claim_ceiling":"withheld_or_blocked","supported_claim_classes":[],
                "blocked_claim_classes":blocked_claims()}),
            "final_packet_proof_source_audit_status_missing",
        ),
        (
            json!({"status":"pending","target_revision":{"kind":"package_digest","value":current}}),
            "final_packet_proof_source_audit_status_unknown:pending",
        ),
        (
            json!({"status":"fail","target_revision":{"kind":"package_digest",
                "value":crate::self_tests::boundaries::workspace_fixtures::sha('d')},
                "claim_ceiling":"withheld_or_blocked","supported_claim_classes":[],
                "blocked_claim_classes":blocked_claims()}),
            "final_packet_proof_source_audit_target_digest_mismatch",
        ),
        (
            json!({"status":"pass","target_revision":{"kind":"package_digest","value":current},
                "claim_ceiling":"withheld_or_blocked","supported_claim_classes":[],
                "blocked_claim_classes":blocked_claims()}),
            "final_packet_proof_source_audit_claim_ceiling_not_source_local",
        ),
        (
            json!({"status":"fail","target_revision":{"kind":"package_digest","value":current},
                "claim_ceiling":"withheld_or_blocked",
                "supported_claim_classes":["source_local_audit_checks"],
                "blocked_claim_classes":blocked_claims()}),
            "final_packet_proof_source_audit_fail_supported_claims_present",
        ),
    ]
}

pub(super) fn blocked_claims() -> Value {
    json!([
        "completion",
        "package_readiness",
        "review_readiness",
        "release_readiness",
        "final_packet_correctness",
        "update_goal_eligibility",
        "app_registry_or_reviewer_exposure"
    ])
}

pub(super) fn assert_stale_and_malformed_source_failures(
    root: &Path,
    store: &crate::schema_catalog::SchemaStore,
    receipt: Value,
    failed_source: Value,
    current: &str,
) {
    let source_path = "validation_artifacts/ultragoal-audit/validator-receipt.json";
    let mut stale_source = failed_source;
    stale_source["source_audit"]["digest"] = json!(crate::digest::ZERO);
    super::assert_guard_failure(
        root,
        store,
        stale_source,
        "final_packet_proof_ref_digest_mismatch:source_audit",
    );
    for source in [
        json!({"status":"pending","target_revision":{"kind":"package_digest","value":current}}),
        json!({"status":"pass","target_revision":{"kind":"package_digest","value":"sha256:wrong"}}),
    ] {
        super::super::super::receipt_fixtures::write_json(&root.join(source_path), &source);
        super::assert_guard_failure(
            root,
            store,
            receipt.clone(),
            "final_packet_proof_ref_digest_mismatch:source_audit",
        );
    }
    let mut malformed_source = receipt;
    std::fs::write(root.join(source_path), "{").expect("malformed source audit");
    malformed_source["source_audit"]["digest"] =
        json!(crate::digest::file(&root.join(source_path)).expect("malformed source digest"));
    malformed_source["source_audit"]["path"] = json!(source_path);
    super::assert_guard_failure(
        root,
        store,
        malformed_source,
        "final_packet_proof_ref_malformed:source_audit",
    );
}
