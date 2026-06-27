use serde_json::Value;
use std::path::Path;

pub(crate) fn canonical_package_failures(root: &Path, receipt: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if pointer_string(receipt, "/claim/id") != "CLAIM-001" {
        out.push("product_fitness_receipt_wrong_claim_id".to_string());
    }
    out.extend(failures(root, receipt));
    out
}

pub(crate) fn failures(root: &Path, receipt: &Value) -> Vec<String> {
    let mut out = required_field_failures(receipt);
    crate::audit::product::fitness::evidence::failures(root, receipt, "", &mut out);
    out.extend(journey_and_evidence_failures(receipt));
    out.extend(metric_failures(receipt));
    out.extend(burden_and_continuance_failures(receipt));
    out.extend(authority_failures(root, receipt));
    out.extend(crate::audit::product::fitness::substitutions::failures(
        receipt,
    ));
    out
}

pub(crate) fn canonical_digest(receipt: &Value) -> String {
    let mut canonical = receipt.clone();
    if let Some(obj) = canonical.as_object_mut() {
        obj.insert(
            "receipt_digest".to_string(),
            Value::String(crate::digest::ZERO.to_string()),
        );
    }
    crate::digest::bytes(&serde_json::to_vec(&canonical).unwrap_or_default())
}

fn required_field_failures(receipt: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if string(receipt, "schema") != "harness-ultragoal.product-fitness-receipt.v1" {
        out.push("product_fitness_receipt_malformed:schema".to_string());
    }
    for (ptr, error) in required_fields() {
        if pointer_string(receipt, ptr).is_empty() {
            out.push((*error).to_string());
        }
    }
    if pointer_string(receipt, "/target_audience/name")
        .to_ascii_lowercase()
        .contains("generic")
    {
        out.push("product_fitness_generic_user_claim".to_string());
    }
    out
}

fn required_fields() -> &'static [(&'static str, &'static str)] {
    &[
        ("/claim/id", "product_fitness_receipt_wrong_claim_id"),
        ("/target_revision/value", "product_fitness_receipt_stale"),
        ("/target_audience/name", "product_fitness_audience_missing"),
        ("/job_to_be_done/job", "product_fitness_job_missing"),
        ("/context_of_use/context", "product_fitness_context_missing"),
        (
            "/desired_user_outcome/outcome",
            "product_fitness_outcome_missing",
        ),
        (
            "/business_or_mission_outcome/outcome",
            "product_fitness_business_or_mission_outcome_missing",
        ),
        (
            "/critical_journey/id",
            "product_fitness_context_of_use_unproven",
        ),
        (
            "/proof_surface/kind",
            "product_fitness_quality_in_use_receipt_missing",
        ),
        ("/claim_ceiling", "product_fitness_claim_ceiling_missing"),
        ("/producer_actor_id", "product_fitness_receipt_malformed"),
        ("/reviewer_actor_id", "product_fitness_receipt_malformed"),
        ("/receipt_digest", "product_fitness_receipt_malformed"),
    ]
}

fn journey_and_evidence_failures(receipt: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if !evidence_present(receipt.pointer("/first_value_event/evidence")) {
        out.push("product_fitness_output_substituted_for_outcome".to_string());
    }
    if receipt
        .pointer("/assumption_tests")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty)
    {
        out.push("product_fitness_assumption_test_missing".to_string());
    }
    if receipt
        .pointer("/user_evidence")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty)
    {
        out.push("product_fitness_unbound_user_evidence".to_string());
    }
    out
}

fn metric_failures(receipt: &Value) -> Vec<String> {
    let metrics = receipt
        .pointer("/quality_in_use_metrics")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut out = Vec::new();
    if metrics.len() < 5 {
        out.push("product_fitness_single_metric_theater".to_string());
    }
    for required in [
        "effectiveness",
        "efficiency",
        "satisfaction",
        "freedom_from_risk",
        "context_coverage",
    ] {
        if !metrics.iter().any(|row| {
            string(row, "dimension") == required && evidence_present(row.get("evidence"))
        }) {
            out.push(format!("product_fitness_quality_in_use_missing:{required}"));
        }
    }
    out
}

fn burden_and_continuance_failures(receipt: &Value) -> Vec<String> {
    let mut out = accessibility_failures(receipt);
    if string(
        receipt
            .pointer("/cognitive_load_gate")
            .unwrap_or(&Value::Null),
        "burden_assessment",
    )
    .is_empty()
    {
        out.push("product_fitness_developer_burden_unmeasured".to_string());
    }
    if string(
        receipt
            .pointer("/recovery_burden_gate")
            .unwrap_or(&Value::Null),
        "recovery_path",
    )
    .is_empty()
    {
        out.push("product_fitness_recovery_burden_missing".to_string());
    }
    let repeated = receipt
        .pointer("/claim/repeated_use_claimed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if repeated && !evidence_present(receipt.pointer("/continuance_signal/evidence")) {
        out.push("product_fitness_continuance_missing".to_string());
    }
    out
}

fn accessibility_failures(receipt: &Value) -> Vec<String> {
    let accessibility = receipt
        .pointer("/accessibility_gate")
        .unwrap_or(&Value::Null);
    let journey_binding = string(accessibility, "journey_binding");
    if journey_binding.is_empty() {
        return vec!["product_fitness_accessibility_missing".to_string()];
    }
    if journey_binding != pointer_string(receipt, "/critical_journey/id") {
        vec!["product_fitness_accessibility_unbound_to_journey".to_string()]
    } else {
        Vec::new()
    }
}

fn authority_failures(root: &Path, receipt: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if string(receipt, "producer_actor_id") == string(receipt, "reviewer_actor_id") {
        out.push("product_fitness_actor_nondisjoint".to_string());
    }
    if string(receipt, "receipt_digest") == crate::digest::ZERO {
        out.push("product_fitness_receipt_malformed:zero_digest".to_string());
    }
    if string(receipt, "receipt_digest") != canonical_digest(receipt) {
        out.push("product_fitness_receipt_digest_mismatch".to_string());
    }
    match crate::package::inventory::package_digest(root) {
        Ok(current) if pointer_string(receipt, "/target_revision/value") == current => {}
        Ok(_) => out.push("product_fitness_receipt_stale".to_string()),
        Err(err) => out.push(format!("product_fitness_digest_unavailable:{err}")),
    }
    out
}

fn pointer_string(value: &Value, pointer: &str) -> String {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn evidence_present(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_object)
        .and_then(|obj| obj.get("path"))
        .and_then(Value::as_str)
        .is_some_and(|path| !path.is_empty())
}

fn string(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}
