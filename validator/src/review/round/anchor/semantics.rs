use super::{artifact::JsonArtifact, policy::AnchorSource};
use serde_json::Value;
use std::path::Path;

pub(crate) struct Inputs<'a> {
    pub(crate) source: AnchorSource,
    pub(crate) root: &'a Path,
    pub(crate) labels: &'a [String; 3],
    pub(crate) validator: &'a JsonArtifact,
    pub(crate) review: &'a JsonArtifact,
    pub(crate) archive: &'a JsonArtifact,
}

pub(crate) fn errors(input: Inputs<'_>) -> Vec<String> {
    let mut out = Vec::new();
    schema_status_errors(&input, &mut out);
    let package = text(&input.validator.value, "/target_revision/value");
    if text(&input.validator.value, "/target_revision/kind") != "package_digest"
        || !digest_valid(package, input.source)
    {
        out.push("review_round_candidate_package_identity_invalid".into());
    }
    let run = text(&input.validator.value, "/run_id");
    if run.is_empty() || run.len() > 256 || run.chars().any(char::is_control) {
        out.push("review_round_validator_run_invalid".into());
    }
    if text(&input.review.value, "/package_digest") != package
        || text(&input.archive.value, "/source/package_digest") != package
    {
        out.push("review_round_anchor_candidate_mismatch".into());
    }
    if text(&input.review.value, "/validator_receipt/path") != input.labels[0]
        || text(&input.review.value, "/validator_receipt/digest") != input.validator.digest
    {
        out.push("review_round_anchor_validator_binding_mismatch".into());
    }
    for digest in [
        text(&input.review.value, "/review_target_digest"),
        text(&input.archive.value, "/archive/digest"),
    ] {
        if !digest_valid(digest, input.source) {
            out.push("review_round_anchor_placeholder_digest".into());
        }
    }
    if input.source == AnchorSource::Live {
        live_errors(&input, package, &mut out);
    }
    out.sort();
    out.dedup();
    out
}

fn schema_status_errors(input: &Inputs<'_>, out: &mut Vec<String>) {
    for (value, schema) in [
        (
            &input.validator.value,
            "harness-ultragoal.validator-receipt.v1",
        ),
        (
            &input.review.value,
            "harness-ultragoal.review-target-receipt.v1",
        ),
        (
            &input.archive.value,
            "harness-ultragoal.distribution-archive-receipt.v1",
        ),
    ] {
        if text(value, "/schema") != schema || text(value, "/status") != "pass" {
            out.push("review_round_anchor_typed_receipt_invalid".into());
        }
    }
}

fn live_errors(input: &Inputs<'_>, package: &str, out: &mut Vec<String>) {
    if text(&input.archive.value, "/archive_purpose") != "candidate_review_anchor" {
        out.push("review_round_archive_purpose_invalid".into());
    }
    match crate::package::inventory::package_digest(input.root) {
        Ok(current) if current == package => {}
        Ok(_) => out.push("review_round_anchor_candidate_mismatch".into()),
        Err(_) => out.push("review_round_candidate_package_unverified".into()),
    }
}

fn digest_valid(value: &str, source: AnchorSource) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    let formatted = hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'));
    formatted
        && (source == AnchorSource::StaticFixture
            || hex.as_bytes().windows(2).any(|pair| pair[0] != pair[1]))
}

fn text<'a>(value: &'a Value, pointer: &str) -> &'a str {
    value.pointer(pointer).and_then(Value::as_str).unwrap_or("")
}
