use serde_json::{Value, json};

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
                "value":crate::self_tests::boundaries::support::sha('d')},
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
