use crate::{review::round::ReviewFailure, review::round::anchor::values::AnchorValues};
use serde_json::Value;

pub(crate) fn row_policy_errors(
    row: &Value,
    receipt: &Value,
    anchors: &AnchorValues,
    persona: &str,
    out: &mut Vec<ReviewFailure>,
) {
    let review_stage = receipt
        .get("round_phase")
        .and_then(Value::as_str)
        .unwrap_or("");
    freshness_errors(row, persona, out);
    scope_error(row, persona, out);
    verdict_error(row, review_stage, persona, out);
    runtime_config_errors(row, review_stage, persona, out);
    anchor_digest_errors(row, receipt, anchors, persona, out);
}

fn freshness_errors(row: &Value, persona: &str, out: &mut Vec<ReviewFailure>) {
    for (key, want) in [("fresh_context", true), ("reused_reviewer", false)] {
        if row.get(key).and_then(Value::as_bool) != Some(want) {
            out.push(ReviewFailure::new(
                "validator-execution-provenance",
                "review_round_reused_reviewer",
                persona,
            ));
        }
    }
}

fn scope_error(row: &Value, persona: &str, out: &mut Vec<ReviewFailure>) {
    if row.get("review_scope").and_then(Value::as_str) != Some("full_current_scope") {
        out.push(ReviewFailure::new(
            "validator-execution-provenance",
            "review_round_not_full_scope",
            persona,
        ));
    }
}

fn verdict_error(row: &Value, review_stage: &str, persona: &str, out: &mut Vec<ReviewFailure>) {
    if row.get("verdict").and_then(Value::as_str)
        != Some(crate::review::round::config::expected_verdict(review_stage))
    {
        out.push(ReviewFailure::new(
            "validator-execution-provenance",
            "review_round_wrong_phase_verdict",
            persona,
        ));
    }
}

fn runtime_config_errors(
    row: &Value,
    review_stage: &str,
    persona: &str,
    out: &mut Vec<ReviewFailure>,
) {
    for (key, want, code) in [
        (
            "model",
            crate::review::round::config::expected_model(review_stage),
            "review_round_model_mismatch",
        ),
        (
            "reasoning_effort",
            "high",
            "review_round_reviewer_effort_mismatch",
        ),
        ("sandbox_mode", "read-only", "review_round_sandbox_mismatch"),
    ] {
        if row.get(key).and_then(Value::as_str) != Some(want) {
            out.push(ReviewFailure::new(
                "validator-execution-provenance",
                code,
                persona,
            ));
        }
    }
}

fn anchor_digest_errors(
    row: &Value,
    receipt: &Value,
    anchors: &AnchorValues,
    persona: &str,
    out: &mut Vec<ReviewFailure>,
) {
    let review_stage = receipt
        .get("round_phase")
        .and_then(Value::as_str)
        .unwrap_or("");
    let anchor_policy = receipt
        .get("anchor_policy")
        .and_then(Value::as_str)
        .unwrap_or("");
    let mut required = vec![("validator_receipt_digest", &anchors.validator_digest)];
    if crate::review::round::config::full_anchor_required(review_stage, anchor_policy) {
        required.push(("review_target_digest", &anchors.review_target_digest));
        required.push(("archive_digest", &anchors.archive_digest));
    }
    for (key, digest) in required {
        if row.get(key).and_then(Value::as_str) != Some(digest) {
            out.push(ReviewFailure::new(
                "validator-execution-provenance",
                "review_round_stale_anchor_digest",
                persona,
            ));
        }
    }
}
