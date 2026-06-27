use crate::schema_catalog;
use crate::target_repo::artifact_refs;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

const REQUIRED: &[&str] = &[
    "docs/product-cohesion.md",
    "validation_artifacts/product-cohesion/journey-receipt.json",
];

pub fn check(
    repo: &Path,
    markers: &[String],
    required: bool,
    checks: &mut serde_json::Map<String, Value>,
) {
    let marker_set = markers.iter().cloned().collect::<BTreeSet<_>>();
    let missing = missing(repo);
    let root = product_root(repo);
    let error = if missing.is_empty() {
        product_error(repo)
    } else {
        None
    };
    let marker = marker_set.contains("product-cohesion");
    let (status, detail) = if required && (!missing.is_empty() || !marker || error.is_some()) {
        (
            "blocked",
            format!(
                "requested product cohesion gate missing: {}",
                missing
                    .first()
                    .cloned()
                    .or(error)
                    .unwrap_or_else(|| "gate marker".to_string())
            ),
        )
    } else if root.is_some() {
        let ok = marker && error.is_none();
        (
            if ok { "pass" } else { "fail" },
            if ok {
                "product cohesion journey receipt complete".to_string()
            } else {
                error.unwrap_or_else(|| "product cohesion marker missing".to_string())
            },
        )
    } else {
        (
            "not_applicable",
            "product cohesion not requested for this target".to_string(),
        )
    };
    checks.insert(
        "product-cohesion".to_string(),
        crate::target_repo::row(repo, status, &detail, root.as_deref()),
    );
}

fn product_root(repo: &Path) -> Option<String> {
    if REQUIRED
        .iter()
        .all(|rel| crate::target_repo::safe_fs::is_nonempty_file(repo, rel))
    {
        Some("validation_artifacts/product-cohesion".to_string())
    } else {
        None
    }
}

fn missing(repo: &Path) -> Vec<String> {
    REQUIRED
        .iter()
        .filter(|rel| !crate::target_repo::safe_fs::is_nonempty_file(repo, rel))
        .map(|rel| rel.to_string())
        .collect()
}

fn product_error(repo: &Path) -> Option<String> {
    let bytes = match crate::target_repo::safe_fs::read(
        repo,
        "validation_artifacts/product-cohesion/journey-receipt.json",
    ) {
        Ok(bytes) => bytes,
        Err(err) => return Some(format!("journey receipt unreadable: {err}")),
    };
    let receipt: Value = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(err) => return Some(format!("journey receipt malformed: {err}")),
    };
    if let Some(error) = schema_catalog::product_cohesion_receipt_errors(&receipt).first() {
        return Some(format!("journey receipt schema invalid: {error}"));
    }
    if receipt
        .pointer("/review/signoff_status")
        .and_then(Value::as_str)
        != Some("pass")
    {
        return Some("journey receipt review signoff not pass".to_string());
    }
    artifact_errors(repo, &receipt)
        .or_else(|| crate::target_repo::product::review::review_error(repo, &receipt))
        .or_else(|| human_attention_error(repo, &receipt))
}

fn artifact_errors(repo: &Path, receipt: &Value) -> Option<String> {
    for (label, item) in artifact_refs_for(receipt) {
        if let Some(error) = artifact_refs::artifact_ref_error(repo, item, &label) {
            return Some(error);
        }
    }
    None
}

fn artifact_refs_for(receipt: &Value) -> Vec<(String, &Value)> {
    let mut refs = Vec::new();
    for (index, step) in receipt
        .pointer("/primary_journey/steps")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
    {
        refs.push((
            format!("journey step {} evidence", index + 1),
            &step["evidence"],
        ));
    }
    for group in ["ui_evidence", "runtime_evidence", "accessibility_evidence"] {
        for (index, item) in receipt
            .pointer(&format!("/proof/{group}"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .enumerate()
        {
            refs.push((format!("{group} {}", index + 1), item));
        }
    }
    for (index, item) in receipt
        .pointer("/human_attention_policy/exhausted_harness_paths")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
    {
        refs.push((format!("exhausted_harness_paths {}", index + 1), item));
    }
    refs.push((
        "reviewer_authority evidence".to_string(),
        &receipt["review"]["reviewer_authority"]["evidence"],
    ));
    if !receipt
        .pointer("/human_attention_policy/human_review_queue_exception")
        .is_none_or(Value::is_null)
    {
        refs.push((
            "human_review_queue_exception evidence".to_string(),
            &receipt["human_attention_policy"]["human_review_queue_exception"]["evidence"],
        ));
    }
    refs
}

fn human_attention_error(repo: &Path, receipt: &Value) -> Option<String> {
    let attention = &receipt["human_attention_policy"];
    let rate = attention
        .get("expected_interruption_rate")
        .and_then(Value::as_str)
        .unwrap_or("");
    let exception = &attention["human_review_queue_exception"];
    if ["frequent", "unknown"].contains(&rate) && exception.is_null() {
        return Some(
            "human attention interruption rate requires human review queue exception".to_string(),
        );
    }
    if exception.is_null() {
        return None;
    }
    let payload = match artifact_refs::read_artifact_json(
        repo,
        &exception["evidence"],
        "human_review_queue_exception evidence",
    ) {
        Ok(value) => value,
        Err(err) => return Some(err),
    };
    if payload.get("schema").and_then(Value::as_str)
        != Some("harness-ultragoal.human-review-queue-exception.v1")
    {
        return Some("human_review_queue_exception evidence schema mismatch".to_string());
    }
    if payload.get("status").and_then(Value::as_str) != Some("pass") {
        return Some("human_review_queue_exception evidence status not pass".to_string());
    }
    if payload.get("product_surface_id") != receipt.get("product_surface_id") {
        return Some("human_review_queue_exception product surface mismatch".to_string());
    }
    for field in [
        "intrinsic_human_decision",
        "scope",
        "owner",
        "why_automation_is_inappropriate",
        "reviewer_authority",
        "review_policy",
        "expires_at",
    ] {
        if payload
            .get(field)
            .and_then(Value::as_str)
            .is_none_or(|text| text.trim().len() < 12)
        {
            return Some(format!(
                "human_review_queue_exception missing substantive {field}"
            ));
        }
    }
    None
}
