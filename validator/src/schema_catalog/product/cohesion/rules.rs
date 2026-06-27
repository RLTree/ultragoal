use serde_json::Value;

use crate::schema_catalog::product::cohesion::value::rules::{
    array_value, artifact, nonempty, required, string, string_array, string_array_min,
};

pub fn product_cohesion_errors(value: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    required(
        value,
        &[
            "schema",
            "product_surface_id",
            "change_intent",
            "primary_user",
            "user_job",
            "product_promise",
            "business_or_user_outcome",
            "primary_journey",
            "surface_map",
            "human_attention_policy",
            "proof",
            "claim_ceiling",
            "review",
            "generated_at",
        ],
        "",
        &mut errors,
    );
    if string(value, "schema") != "harness-ultragoal.product-cohesion-receipt.v1" {
        errors.push("schema must be product-cohesion-receipt.v1".to_string());
    }
    for key in [
        "product_surface_id",
        "change_intent",
        "primary_user",
        "user_job",
        "product_promise",
        "business_or_user_outcome",
        "generated_at",
    ] {
        nonempty(value, key, key, &mut errors);
    }
    journey_errors(&value["primary_journey"], &mut errors);
    surface_map_errors(&value["surface_map"], &mut errors);
    human_attention_errors(&value["human_attention_policy"], &mut errors);
    proof_errors(&value["proof"], &mut errors);
    claim_ceiling_errors(&value["claim_ceiling"], &mut errors);
    review_errors(&value["review"], &mut errors);
    errors
}

fn journey_errors(value: &Value, errors: &mut Vec<String>) {
    required(
        value,
        &["id", "start", "end", "steps"],
        "primary_journey",
        errors,
    );
    for key in ["id", "start", "end"] {
        nonempty(value, key, &format!("primary_journey.{key}"), errors);
    }
    let steps = array_value(&value["steps"], "primary_journey.steps", 2, errors);
    for (index, step) in steps.iter().enumerate() {
        let path = format!("primary_journey.steps[{index}]");
        required(
            step,
            &[
                "surface",
                "user_action",
                "system_response",
                "engine_truth",
                "evidence",
            ],
            &path,
            errors,
        );
        for key in ["surface", "user_action", "system_response", "engine_truth"] {
            nonempty(step, key, &format!("{path}.{key}"), errors);
        }
        artifact(&step["evidence"], &format!("{path}.evidence"), errors);
    }
}

fn surface_map_errors(value: &Value, errors: &mut Vec<String>) {
    let rows = array_value(value, "surface_map", 1, errors);
    for (index, row) in rows.iter().enumerate() {
        let path = format!("surface_map[{index}]");
        required(row, &["surface", "role", "state_coverage"], &path, errors);
        nonempty(row, "surface", &format!("{path}.surface"), errors);
        nonempty(row, "role", &format!("{path}.role"), errors);
        string_array_min(
            &row["state_coverage"],
            &format!("{path}.state_coverage"),
            1,
            errors,
        );
    }
}

fn human_attention_errors(value: &Value, errors: &mut Vec<String>) {
    required(
        value,
        &[
            "expected_interruption_rate",
            "allowed_reasons",
            "automation_expectation",
            "overuse_risk",
            "exhausted_harness_paths",
        ],
        "human_attention_policy",
        errors,
    );
    if !["rare", "occasional", "frequent", "unknown"]
        .contains(&string(value, "expected_interruption_rate").as_str())
    {
        errors.push("human_attention_policy.expected_interruption_rate invalid".to_string());
    }
    string_array_min(
        &value["allowed_reasons"],
        "human_attention_policy.allowed_reasons",
        1,
        errors,
    );
    nonempty(
        value,
        "automation_expectation",
        "human_attention_policy.automation_expectation",
        errors,
    );
    nonempty(
        value,
        "overuse_risk",
        "human_attention_policy.overuse_risk",
        errors,
    );
    for (index, item) in array_value(
        &value["exhausted_harness_paths"],
        "human_attention_policy.exhausted_harness_paths",
        1,
        errors,
    )
    .iter()
    .enumerate()
    {
        artifact(
            item,
            &format!("human_attention_policy.exhausted_harness_paths[{index}]"),
            errors,
        );
    }
}

fn proof_errors(value: &Value, errors: &mut Vec<String>) {
    required(
        value,
        &["ui_evidence", "runtime_evidence", "accessibility_evidence"],
        "proof",
        errors,
    );
    for group in ["ui_evidence", "runtime_evidence"] {
        for (index, item) in array_value(&value[group], &format!("proof.{group}"), 1, errors)
            .iter()
            .enumerate()
        {
            artifact(item, &format!("proof.{group}[{index}]"), errors);
        }
    }
    for (index, item) in value["accessibility_evidence"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        artifact(
            item,
            &format!("proof.accessibility_evidence[{index}]"),
            errors,
        );
    }
}

fn claim_ceiling_errors(value: &Value, errors: &mut Vec<String>) {
    required(
        value,
        &["supported", "withheld", "rationale"],
        "claim_ceiling",
        errors,
    );
    string_array(&value["supported"], "claim_ceiling.supported", errors);
    string_array(&value["withheld"], "claim_ceiling.withheld", errors);
    nonempty(value, "rationale", "claim_ceiling.rationale", errors);
}

fn review_errors(value: &Value, errors: &mut Vec<String>) {
    required(
        value,
        &[
            "method",
            "reviewers",
            "reviewer_authority",
            "signoff_status",
        ],
        "review",
        errors,
    );
    nonempty(value, "method", "review.method", errors);
    string_array_min(&value["reviewers"], "review.reviewers", 1, errors);
    required(
        &value["reviewer_authority"],
        &["actor_disjoint", "authority", "evidence"],
        "review.reviewer_authority",
        errors,
    );
    if value
        .pointer("/reviewer_authority/actor_disjoint")
        .and_then(Value::as_bool)
        != Some(true)
    {
        errors.push("review.reviewer_authority.actor_disjoint must be true".to_string());
    }
    artifact(
        &value["reviewer_authority"]["evidence"],
        "review.reviewer_authority.evidence",
        errors,
    );
    if !["pass", "revise", "blocked"].contains(&string(value, "signoff_status").as_str()) {
        errors.push("review.signoff_status invalid".to_string());
    }
}
