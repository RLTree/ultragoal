use serde_json::{Value, json};
use std::path::Path;

pub(super) fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

pub(super) fn production_law(check_id: &str) -> Value {
    json!({
        "law_id":"schema-valid",
        "schema":"harness-ultragoal.mandatory-law-surface-receipt.v1",
        "enforcement_status":"deterministic_fail_closed",
        "standards_row_id":"schema-valid",
        "source_obligation_id":"schema-valid",
        "foundational_trace_id":"schema-valid",
        "validator_check_id":check_id,
        "valid_fixture_path":"fixtures/mandatory-law-surfaces/valid/schema-valid.json",
        "claim_ceiling_guard":"blocks",
        "red_fixture_ids":["schema-valid-red"],
        "behavior_failure_modes":["schema-invalid"],
        "law_specific":{"schema_dispatch":true}
    })
}

pub(super) fn with_independent_verification(mut law: Value) -> Value {
    law["independent_verification"] = json!({
        "authority":"parent_verified_source_runtime",
        "cli_pass_alone_allowed":false,
        "source_runtime_manual_required":true,
        "verification_scope":"per_law",
        "receipt_path":"validation_artifacts/manual/parent-source-runtime-verification.json"
    });
    law
}

pub(super) fn validator_theater_law() -> Value {
    let mut law = production_law("validator-theater-miswire-resistance");
    law["law_id"] = json!("validator-theater-miswire-resistance");
    law["standards_row_id"] = json!("validator-theater-miswire-resistance");
    law["source_obligation_id"] = json!("validator-theater-miswire-resistance");
    law["foundational_trace_id"] = json!("validator-theater-miswire-resistance");
    law["valid_fixture_path"] =
        json!("fixtures/mandatory-law-surfaces/valid/validator-theater-miswire-resistance.json");
    law["red_fixture_ids"] = json!(["validator-theater-miswire-resistance-red"]);
    law["behavior_failure_modes"] = json!(["validator-theater-cli-pass-without-source-runtime"]);
    law["law_specific"] = json!({"cli_pass_without_parent_source_runtime_verification": true});
    law
}

pub(super) fn write_specific_red_fixture(root: &Path, id: &str, law: &str, field: &str) {
    write_json(
        &root.join("fixtures/red").join(format!("{id}.json")),
        &json!({
            "schema": "harness-ultragoal.red-packet.v1",
            "id": id,
            "expected_failure": {
                "check_id": law,
                "error": format!("mandatory_law_specific_guard_not_enforced:{law}:{field}")
            }
        }),
    );
}

pub(super) fn write_manual_verification(root: &Path, law: &str) {
    let candidate = crate::package::inventory::package_digest(root).expect("candidate");
    write_json(
        &root.join("validation_artifacts/manual/parent-source-runtime-verification.json"),
        &json!({
            "schema": "harness-ultragoal.parent-source-runtime-verification.v1",
            "status": "verified_current",
            "candidate_digest": candidate,
            "law_ids": [law],
            "cli_pass_alone_rejected": true,
            "source_paths": ["validator/src/audit/mandatory/law/surfaces/mod.rs"],
            "runtime_evidence_paths": [
                "validation_artifacts/manual/parent-source-runtime-verification.json"
            ],
            "manual_checks": [{
                "claim_path": "mandatory-law-surfaces",
                "verification_method": "source_inspection",
                "finding": "source and receipt inspected independently from CLI pass output"
            }]
        }),
    );
}

pub(super) fn expect_failure(root: &Path, receipt: &Value, expected: &str) {
    let failures = crate::audit::mandatory::law::surfaces::receipt_value_failures(root, receipt);
    assert!(
        failures.iter().any(|failure| failure.contains(expected)),
        "{expected}: {failures:?}"
    );
}
