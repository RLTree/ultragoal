pub(crate) mod anchor;
pub(crate) mod artifacts;
pub(crate) mod claim;
pub(crate) mod config;
pub(crate) mod personas;
pub(crate) mod product;
pub(crate) mod registry;
pub(crate) mod report;
pub(crate) mod row;
pub(crate) mod runtime;
pub(crate) mod spawn;

use crate::{json_boundary, schema_catalog};
use serde_json::Value;
use std::path::Path;

pub use anchor::values::AnchorPaths;

const SCHEMA: &str = "harness-ultragoal.review-round-receipt.v2";

pub fn validate_files(root: &Path, receipt: &Path, anchors: &AnchorPaths) -> Result<(), String> {
    let value = json_boundary::read_json(receipt)?;
    let anchor_values = crate::review::round::anchor::values::AnchorValues::read(
        Some(root),
        &anchors.validator_receipt,
        &anchors.review_target_receipt,
        &anchors.archive_receipt,
    )?;
    let store = schema_catalog::load(root);
    let failures = receipt_errors(root, &store, &value, &anchor_values);
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures
            .into_iter()
            .map(|failure| format!("{}: {}", failure.error, failure.detail))
            .collect::<Vec<_>>()
            .join("; "))
    }
}

pub fn fixture_failures(root: &Path) -> Vec<String> {
    let store = schema_catalog::load(root);
    let receipt_path = root.join("fixtures/review-round/valid/review-round-receipt.json");
    let anchors = crate::review::round::anchor::values::fixture_anchor_values(root);
    match json_boundary::read_json(&receipt_path) {
        Ok(value) => receipt_errors(root, &store, &value, &anchors)
            .into_iter()
            .map(|failure| format!("review-round fixture: {}", failure.error))
            .collect(),
        Err(err) => vec![format!("review-round fixture unreadable: {err}")],
    }
}

pub fn red_errors(
    root: &Path,
    store: &schema_catalog::SchemaStore,
    value: &Value,
) -> Vec<ReviewFailure> {
    let anchors = crate::review::round::anchor::values::fixture_anchor_values(root);
    receipt_errors(root, store, value, &anchors)
}

pub(crate) fn red_errors_with_anchors(
    root: &Path,
    store: &schema_catalog::SchemaStore,
    value: &Value,
    anchors: &crate::review::round::anchor::values::AnchorValues,
) -> Vec<ReviewFailure> {
    receipt_errors(root, store, value, anchors)
}

pub fn is_review_round_fixture(path: &str) -> bool {
    path == "fixtures/review-round/valid/review-round-receipt.json"
}

fn receipt_errors(
    root: &Path,
    store: &schema_catalog::SchemaStore,
    value: &Value,
    anchors: &crate::review::round::anchor::values::AnchorValues,
) -> Vec<ReviewFailure> {
    let mut out = schema_catalog::schema_errors(store, "review-round-receipt.schema.json", value)
        .into_iter()
        .map(|error| ReviewFailure::new("schema-valid", "schema_validation_failed", error))
        .collect::<Vec<_>>();
    semantic_errors(root, value, anchors, &mut out);
    out
}

fn semantic_errors(
    root: &Path,
    value: &Value,
    anchors: &crate::review::round::anchor::values::AnchorValues,
    out: &mut Vec<ReviewFailure>,
) {
    if value.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
        out.push(ReviewFailure::new(
            "validator-execution-provenance",
            "review_round_schema_invalid",
            "schema",
        ));
    }
    crate::review::round::anchor::values::anchor_errors(value, anchors, out);
    crate::review::materiality::review_round_errors(root, value, anchors, out);
    crate::review::round::runtime::receipt_errors(value, out);
    crate::review::round::personas::persona_errors(root, value, anchors, out);
}

#[derive(Clone)]
pub struct ReviewFailure {
    pub check: String,
    pub error: String,
    pub detail: String,
}

impl ReviewFailure {
    pub(crate) fn new(check: &str, error: &str, detail: impl Into<String>) -> Self {
        Self {
            check: check.to_string(),
            error: error.to_string(),
            detail: detail.into(),
        }
    }
}
