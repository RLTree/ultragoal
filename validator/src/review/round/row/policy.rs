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
        .get("review_stage")
        .and_then(Value::as_str)
        .unwrap_or("");
    freshness_errors(row, persona, out);
    scope_error(row, persona, out);
    falsification_result_error(row, review_stage, persona, out);
    reviewer_authority_errors(row, persona, out);
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

fn falsification_result_error(
    row: &Value,
    review_stage: &str,
    role: &str,
    out: &mut Vec<ReviewFailure>,
) {
    if row.get("falsification_result").and_then(Value::as_str)
        != Some(crate::review::round::config::expected_result(review_stage))
    {
        out.push(ReviewFailure::new(
            "validator-execution-provenance",
            "review_round_wrong_falsification_result",
            role,
        ));
    }
}

fn reviewer_authority_errors(row: &Value, role: &str, out: &mut Vec<ReviewFailure>) {
    for (key, want, code) in [
        (
            "reviewer_authority",
            "falsification_only_cannot_raise_claims",
            "review_round_reviewer_authority_invalid",
        ),
        ("sandbox_mode", "read-only", "review_round_sandbox_mismatch"),
    ] {
        if row.get(key).and_then(Value::as_str) != Some(want) {
            out.push(ReviewFailure::new(
                "validator-execution-provenance",
                code,
                role,
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
        .get("review_stage")
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
