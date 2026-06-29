use crate::cli::observe::telemetry;
use crate::cli::observe::types::ObserveCommand;
use serde_json::{Value, json};
use std::path::Path;

pub(crate) fn run(root: &Path, command: &ObserveCommand) -> Result<Value, String> {
    let audit = root.join("validation_artifacts/ultragoal-audit/validator-receipt.json");
    let known_current_failure = current_failure(root);
    let failure_summary = failure_summary(&known_current_failure);
    let mut receipt = telemetry::base_receipt(root, command, "pass", failure_summary.as_deref())?;
    receipt["explanation"] = json!({
        "requested_run_id": command.run_id,
        "requested_claim_id": command.claim_id,
        "requested_check_id": command.check_id,
        "requested_law_id": command.law_id,
        "current_source_audit_receipt": "validation_artifacts/ultragoal-audit/validator-receipt.json",
        "current_source_audit_digest": crate::digest::file(&audit).unwrap_or_else(|_| crate::digest::ZERO.to_string()),
        "known_current_failure": known_current_failure,
        "repair_guidance": "Inspect final-packet proof/source-audit digest dereference, regenerate same-candidate lower-level receipts, and rerun source audit once after implementation changes."
    });
    Ok(receipt)
}

fn failure_summary(value: &Value) -> Option<String> {
    let failures = value.as_array()?;
    let first = failures.first()?.as_str()?;
    if first == "no current source-audit failure; inspect final-packet/control receipts" {
        return None;
    }
    Some(
        failures
            .iter()
            .filter_map(Value::as_str)
            .take(8)
            .collect::<Vec<_>>()
            .join("; "),
    )
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
    let check_failures: Vec<Value> = check_failures(&value);
    if check_failures.is_empty() {
        json!(["no current source-audit failure; inspect final-packet/control receipts"])
    } else {
        Value::Array(check_failures)
    }
}

fn check_failures(value: &Value) -> Vec<Value> {
    if let Some(rows) = value.get("checks").and_then(Value::as_array) {
        return rows
            .iter()
            .filter(|check| check.get("status").and_then(Value::as_str) != Some("pass"))
            .filter_map(|check| {
                check
                    .get("id")
                    .and_then(Value::as_str)
                    .map(|id| Value::String(id.to_string()))
            })
            .collect();
    }
    value
        .get("checks")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|checks| checks.iter())
        .filter(|(_, check)| check.get("status").and_then(Value::as_str) != Some("pass"))
        .map(|(id, check)| {
            let details = check
                .get("details")
                .and_then(Value::as_str)
                .filter(|text| !text.is_empty() && *text != "pass");
            match details {
                Some(details) => Value::String(format!("{id}: {details}")),
                None => Value::String(id.clone()),
            }
        })
        .collect()
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
        assert!(
            receipt["why_failed"]
                .as_str()
                .unwrap()
                .contains("final_packet_proof_source_audit_target_digest_mismatch")
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
        assert!(
            packet_receipt["why_failed"]
                .as_str()
                .unwrap()
                .contains("plugin_self_law_registry_status_not_pass")
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
        assert!(
            check_receipt["why_failed"]
                .as_str()
                .unwrap()
                .contains("validator-execution-provenance")
        );
        crate::json_boundary::write_json(
            &root.join("validation_artifacts/ultragoal-audit/validator-receipt.json"),
            &json!({
                "failures":[],
                "checks":{
                    "source-audit-pass":{"status":"pass","details":"pass"},
                    "validator-execution-provenance":{
                        "status":"fail",
                        "details":"final_packet_proof_source_audit_target_digest_mismatch"
                    }
                }
            }),
        )
        .expect("audit object checks receipt");
        let object_check_receipt = run(&root, &command).expect("explain object checks");
        assert!(
            object_check_receipt["why_failed"]
                .as_str()
                .unwrap()
                .contains("final_packet_proof_source_audit_target_digest_mismatch")
        );
        crate::json_boundary::write_json(
            &root.join("validation_artifacts/ultragoal-audit/validator-receipt.json"),
            &json!({
                "failures":[],
                "checks":{
                    "source-audit-pass":{"status":"pass","details":"pass"},
                    "validator-execution-provenance":{"status":"fail","details":"pass"}
                }
            }),
        )
        .expect("audit object checks without details receipt");
        let object_no_details_receipt =
            run(&root, &command).expect("explain object checks without details");
        assert_eq!(
            object_no_details_receipt["explanation"]["known_current_failure"][0],
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
