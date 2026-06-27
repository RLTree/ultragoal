use crate::target_repo::artifact_refs;
use serde_json::Value;
use std::path::Path;

const BANNED_REVIEWERS: &[&str] = &["author", "packageauthor", "producer", "fixtureauthor"];

pub fn review_error(repo: &Path, receipt: &Value) -> Option<String> {
    let reviewers = normalized_reviewers(&receipt["review"]["reviewers"]);
    if reviewers.is_empty() {
        return Some("journey receipt review requires at least one reviewer".to_string());
    }
    if reviewers
        .iter()
        .any(|reviewer| BANNED_REVIEWERS.contains(&reviewer.as_str()))
    {
        return Some("journey receipt review must be actor-disjoint".to_string());
    }
    let authority = &receipt["review"]["reviewer_authority"];
    if authority.get("actor_disjoint").and_then(Value::as_bool) != Some(true) {
        return Some("journey receipt review authority must be actor-disjoint".to_string());
    }
    review_evidence_error(repo, receipt, authority, &reviewers)
}

fn review_evidence_error(
    repo: &Path,
    receipt: &Value,
    authority: &Value,
    reviewers: &[String],
) -> Option<String> {
    let payload = match artifact_refs::read_artifact_json(
        repo,
        &authority["evidence"],
        "reviewer authority evidence",
    ) {
        Ok(value) => value,
        Err(err) => return Some(err),
    };
    if payload.get("schema").and_then(Value::as_str)
        != Some("harness-ultragoal.product-cohesion-review.v1")
    {
        return Some("reviewer authority evidence schema mismatch".to_string());
    }
    if payload.get("status").and_then(Value::as_str) != Some("pass") {
        return Some("reviewer authority evidence status not pass".to_string());
    }
    if payload.get("actor_disjoint").and_then(Value::as_bool) != Some(true) {
        return Some("reviewer authority evidence must be actor-disjoint".to_string());
    }
    if payload.get("authority").and_then(Value::as_str)
        != authority.get("authority").and_then(Value::as_str)
    {
        return Some("reviewer authority evidence authority mismatch".to_string());
    }
    if normalized_reviewers(&payload["reviewers"]) != reviewers {
        return Some("reviewer authority evidence reviewers mismatch".to_string());
    }
    if payload.get("method").and_then(Value::as_str)
        != receipt["review"].get("method").and_then(Value::as_str)
    {
        return Some("reviewer authority evidence method mismatch".to_string());
    }
    None
}

fn normalized_reviewers(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(canonical_reviewer)
        .filter(|item| !item.is_empty())
        .collect()
}

fn canonical_reviewer(item: &str) -> String {
    item.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}
