use serde_json::Value;

pub(super) fn required_fields() -> &'static [(&'static str, &'static str)] {
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

pub(super) fn pointer_string(value: &Value, pointer: &str) -> String {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

pub(super) fn evidence_present(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_object)
        .and_then(|obj| obj.get("path"))
        .and_then(Value::as_str)
        .is_some_and(|path| !path.is_empty())
}

pub(super) fn string(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}
