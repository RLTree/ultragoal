use crate::cli::observe::telemetry;
use crate::cli::observe::types::ObserveCommand;
use serde_json::{Value, json};
use std::path::Path;

pub(crate) fn run(root: &Path, command: &ObserveCommand) -> Result<Value, String> {
    let audit = root.join("validation_artifacts/ultragoal-audit/validator-receipt.json");
    let mut receipt = telemetry::base_receipt(root, command, "pass", None)?;
    receipt["explanation"] = json!({
        "requested_run_id": command.run_id,
        "requested_claim_id": command.claim_id,
        "requested_check_id": command.check_id,
        "requested_law_id": command.law_id,
        "current_source_audit_receipt": "validation_artifacts/ultragoal-audit/validator-receipt.json",
        "current_source_audit_digest": crate::digest::file(&audit).unwrap_or_else(|_| crate::digest::ZERO.to_string()),
        "known_current_failure": current_failure(root),
        "repair_guidance": "Inspect final-packet proof/source-audit digest dereference, regenerate same-candidate lower-level receipts, and rerun source audit once after implementation changes."
    });
    Ok(receipt)
}

fn current_failure(root: &Path) -> Value {
    let packet = root.join("validation_artifacts/review/final-packet-proof.json");
    if let Some(failures) = crate::json_boundary::read_json(&packet)
        .ok()
        .and_then(|value| value.pointer("/failure/observed_failures").cloned())
        .filter(|value| value.as_array().is_some_and(|items| !items.is_empty()))
    {
        return failures;
    }
    let path = root.join("validation_artifacts/ultragoal-audit/validator-receipt.json");
    let Some(value) = crate::json_boundary::read_json(&path).ok() else {
        return json!(["source audit receipt unavailable"]);
    };
    if let Some(failures) = value
        .get("failures")
        .cloned()
        .filter(|value| value.as_array().is_some_and(|items| !items.is_empty()))
    {
        return failures;
    }
    let check_failures: Vec<Value> = value
        .get("checks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|check| check.get("status").and_then(Value::as_str) != Some("pass"))
        .filter_map(|check| {
            check
                .get("id")
                .and_then(Value::as_str)
                .map(|id| Value::String(id.to_string()))
        })
        .collect();
    if check_failures.is_empty() {
        json!(["no current source-audit failure; inspect final-packet/control receipts"])
    } else {
        Value::Array(check_failures)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn explain_reports_current_failure_and_bad_root_errors() {
        let root = crate::self_tests::boundaries::support::temp_root("observe-explain");
        fs::create_dir_all(&root).expect("root");
        fs::write(root.join("owned.txt"), "owned").expect("owned");
        crate::json_boundary::write_json(
            &root.join("plugin-manifest-draft.json"),
            &json!({"resources":["owned.txt"]}),
        )
        .expect("manifest");
        fs::create_dir_all(root.join("validation_artifacts/ultragoal-audit")).expect("audit dir");
        crate::json_boundary::write_json(
            &root.join("validation_artifacts/ultragoal-audit/validator-receipt.json"),
            &json!({"failures":["final_packet_proof_source_audit_target_digest_mismatch"]}),
        )
        .expect("audit receipt");
        let command = crate::cli::observe::parse(&[
            "observe".to_string(),
            "explain-failure".to_string(),
            "--run-id".to_string(),
            "run-1".to_string(),
        ])
        .expect("parse")
        .expect("observe");
        let receipt = run(&root, &command).expect("explain");
        assert_eq!(
            receipt["explanation"]["known_current_failure"][0],
            "final_packet_proof_source_audit_target_digest_mismatch"
        );
        fs::create_dir_all(root.join("validation_artifacts/review")).expect("review dir");
        crate::json_boundary::write_json(
            &root.join("validation_artifacts/review/final-packet-proof.json"),
            &json!({
                "status":"fail",
                "failure":{"observed_failures":["final_packet_proof_registry_ref:plugin_self_law_registry_status_not_pass"]}
            }),
        )
        .expect("packet receipt");
        let packet_receipt = run(&root, &command).expect("explain packet");
        assert_eq!(
            packet_receipt["explanation"]["known_current_failure"][0],
            "final_packet_proof_registry_ref:plugin_self_law_registry_status_not_pass"
        );
        fs::remove_file(root.join("validation_artifacts/review/final-packet-proof.json"))
            .expect("remove packet receipt");
        crate::json_boundary::write_json(
            &root.join("validation_artifacts/ultragoal-audit/validator-receipt.json"),
            &json!({
                "failures":[],
                "checks":[
                    {"id":"source-audit-pass","status":"pass"},
                    {"id":"validator-execution-provenance","status":"fail"}
                ]
            }),
        )
        .expect("audit checks receipt");
        let check_receipt = run(&root, &command).expect("explain checks");
        assert_eq!(
            check_receipt["explanation"]["known_current_failure"][0],
            "validator-execution-provenance"
        );
        crate::json_boundary::write_json(
            &root.join("validation_artifacts/ultragoal-audit/validator-receipt.json"),
            &json!({"failures":[],"checks":[{"id":"source-audit-pass","status":"pass"}]}),
        )
        .expect("audit pass receipt");
        let no_failure_receipt = run(&root, &command).expect("explain no failure");
        assert_eq!(
            no_failure_receipt["explanation"]["known_current_failure"][0],
            "no current source-audit failure; inspect final-packet/control receipts"
        );
        fs::remove_file(root.join("validation_artifacts/ultragoal-audit/validator-receipt.json"))
            .expect("remove audit receipt");
        let missing_receipt = run(&root, &command).expect("explain missing audit");
        assert_eq!(
            missing_receipt["explanation"]["known_current_failure"][0],
            "source audit receipt unavailable"
        );
        let bad_root = crate::self_tests::boundaries::support::temp_root("observe-explain-bad");
        fs::create_dir_all(&bad_root).expect("bad root");
        assert!(
            run(&bad_root, &command)
                .unwrap_err()
                .contains("plugin-manifest-draft.json")
        );
        fs::remove_dir_all(root).expect("cleanup explain");
        fs::remove_dir_all(bad_root).expect("cleanup explain bad");
    }
}
