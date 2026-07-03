use crate::review::round::ReviewFailure;
use serde_json::Value;
use std::collections::BTreeSet;

const FULL: &str = "FULL_SCOPE_MATERIAL_REVIEW_REQUIRED";
const DELTA: &str = "DELTA_REVIEW_ALLOWED";
const ADVISORY: &str = "ADVISORY_REVIEW_ALLOWED";
const BLOCKED: &str = "BLOCKED_BEFORE_REVIEW";

const PERSONAS: &[&str] = &[
    "contract_claim_falsifier",
    "orchestration_recovery_falsifier",
    "security_trust_boundary_falsifier",
    "product_simplicity_falsifier",
];

const PREFLIGHT_GATES: &[&str] = &[
    "anchor_existence_and_digest",
    "validator_receipt_status",
    "reviewer_registry_model_persona_exposure",
    "stale_receipt_check",
    "package_private_artifact_hygiene",
    "readiness_validator",
    "claim_ceiling_check",
    "proof_surface_substitution_check",
];

pub(crate) fn review_round_errors(receipt: &Value, out: &mut Vec<ReviewFailure>) {
    let gate = &receipt["materiality_gate"];
    for error in value_failures(gate) {
        out.push(ReviewFailure::new(
            "material-review-scope-gate",
            &error,
            "materiality_gate",
        ));
    }
    let decision = str_field(gate, "decision");
    if receipt.get("review_stage").and_then(Value::as_str) == Some("sign_off") {
        if decision != FULL {
            out.push(failure("material_signoff_requires_full_scope"));
        }
        if gate
            .get("output_may_be_used_for_material_signoff")
            .and_then(Value::as_bool)
            != Some(true)
        {
            out.push(failure("material_signoff_not_authorized"));
        }
    }
}

pub(crate) fn fixture_failures(root: &std::path::Path) -> Vec<String> {
    let dir = root.join("fixtures/review-materiality/valid");
    let Ok(entries) = std::fs::read_dir(dir) else {
        return vec!["review_materiality_fixture_dir_missing".to_string()];
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            (path.extension().and_then(|ext| ext.to_str()) == Some("json")).then_some(path)
        })
        .flat_map(|path| match crate::json_boundary::read_json(&path) {
            Ok(value) => value_failures(&value)
                .into_iter()
                .map(move |error| format!("{}: {error}", path.display()))
                .collect::<Vec<_>>(),
            Err(err) => vec![format!("{}: {err}", path.display())],
        })
        .collect()
}

pub(crate) fn value_failures(value: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let decision = str_field(value, "decision");
    if ![FULL, DELTA, ADVISORY, BLOCKED].contains(&decision.as_str()) {
        out.push("materiality_decision_invalid".to_string());
        return out;
    }
    gate_run_errors(value, &mut out);
    claim_ceiling_errors(value, &mut out);
    if decision == FULL {
        full_errors(value, &mut out);
    } else if decision == DELTA {
        delta_errors(value, &mut out);
    } else if decision == ADVISORY {
        advisory_errors(value, &mut out);
    } else {
        blocked_errors(value, &mut out);
    }
    out
}

fn gate_run_errors(value: &Value, out: &mut Vec<String>) {
    let required = strings(value, "deterministic_gates_required");
    let run = strings(value, "deterministic_gates_run");
    for gate in PREFLIGHT_GATES {
        if !required.contains(*gate) {
            out.push("materiality_required_gate_missing".to_string());
        }
    }
    for gate in required {
        if !run.contains(gate.as_str()) {
            out.push("materiality_required_gate_not_run".to_string());
        }
    }
}

fn claim_ceiling_errors(value: &Value, out: &mut Vec<String>) {
    let Some(ceiling) = value.get("claim_ceiling") else {
        out.push("materiality_claim_ceiling_missing".to_string());
        return;
    };
    for key in ["supported", "unsupported", "blocked"] {
        if !ceiling.get(key).is_some_and(Value::is_array) {
            out.push("materiality_claim_ceiling_missing".to_string());
        }
    }
}

fn full_errors(value: &Value, out: &mut Vec<String>) {
    let reviewers = strings(value, "reviewers_required");
    if PERSONAS.iter().any(|persona| !reviewers.contains(*persona)) || reviewers.len() != 4 {
        out.push("materiality_full_scope_reviewer_set_missing".to_string());
    }
    if value
        .get("output_may_be_used_for_material_signoff")
        .and_then(Value::as_bool)
        != Some(true)
    {
        out.push("materiality_full_scope_not_signoff_capable".to_string());
    }
    if value
        .get("anchors_checked")
        .and_then(Value::as_array)
        .map_or(true, Vec::is_empty)
    {
        out.push("materiality_anchor_evidence_missing".to_string());
    }
    if value
        .get("reviewer_registry_evidence")
        .and_then(Value::as_array)
        .map_or(true, Vec::is_empty)
    {
        out.push("materiality_reviewer_registry_evidence_missing".to_string());
    }
    if strings(value, "material_triggers").is_empty() {
        out.push("materiality_trigger_missing".to_string());
    }
}

fn delta_errors(value: &Value, out: &mut Vec<String>) {
    if value
        .get("output_may_be_used_for_material_signoff")
        .and_then(Value::as_bool)
        != Some(false)
    {
        out.push("materiality_delta_used_for_signoff".to_string());
    }
    if strings(value, "reviewers_required").len() > 1 {
        out.push("materiality_delta_reviewer_scope_too_broad".to_string());
    }
}

fn advisory_errors(value: &Value, out: &mut Vec<String>) {
    if value
        .get("output_may_be_used_for_material_signoff")
        .and_then(Value::as_bool)
        != Some(false)
    {
        out.push("materiality_advisory_used_for_signoff".to_string());
    }
    let supported = value
        .pointer("/claim_ceiling/supported")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    if supported != 0 {
        out.push("materiality_advisory_overclaims_support".to_string());
    }
}

fn blocked_errors(value: &Value, out: &mut Vec<String>) {
    if !strings(value, "reviewers_required").is_empty() {
        out.push("materiality_blocked_launches_reviewers".to_string());
    }
    if strings(value, "validator_repairs_recommended").is_empty() {
        out.push("materiality_blocked_without_repair".to_string());
    }
}

fn failure(error: &str) -> ReviewFailure {
    ReviewFailure::new("material-review-scope-gate", error, "materiality_gate")
}

fn strings(value: &Value, key: &str) -> BTreeSet<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}

fn str_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}
