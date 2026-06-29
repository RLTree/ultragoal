use super::{assert_guard_failure, mutate, rewrite_ref};
use serde_json::{Value, json};
use std::path::Path;

pub(super) fn assert_bad_registry(
    root: &Path,
    store: &crate::schema_catalog::SchemaStore,
    receipt: &Value,
    current: &str,
) {
    assert_guard_failure(
        root,
        store,
        mutate(receipt, |value| {
            value["registry_exposure"]["status"] = json!("pending")
        }),
        "final_packet_proof_ref_embedded_status_not_pass_or_fail:registry",
    );

    let mut bad_registry = receipt.clone();
    rewrite_ref(
        root,
        &mut bad_registry,
        "registry_exposure",
        "validation_artifacts/ultragoal-audit/active-registry-exposure-current.json",
        &json!({
            "schema":"harness-ultragoal.multi-agent-registry-exposure.v1",
            "status":"fail",
            "source":"manual-json",
            "target_revision":{"kind":"package_digest","value":current},
            "claim_ceiling":"withheld_or_blocked",
            "issuer":{"tool":"ultragoal","authority":"cli_control_plane"},
            "capture_method":"fail_closed_no_capability",
            "failure":{"reason":"live_registry_reviewer_exposure_not_proven"},
            "blocked_claim_classes":["completion"]
        }),
    );
    assert_guard_failure(
        root,
        store,
        bad_registry,
        "final_packet_proof_registry_ref:plugin_self_law_registry_guard_wrong_source",
    );
}

pub(super) fn assert_missing_registry_failure(
    root: &Path,
    store: &crate::schema_catalog::SchemaStore,
    receipt: &Value,
) {
    let mut missing_registry = receipt.clone();
    missing_registry
        .as_object_mut()
        .expect("proof object")
        .remove("registry_exposure");
    assert_guard_failure(
        root,
        store,
        missing_registry,
        "final_packet_proof_ref_missing:registry",
    );
}
