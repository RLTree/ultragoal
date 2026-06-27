use crate::{digest, review::round::ReviewFailure};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

pub(crate) fn artifact_binding_errors(
    root: &Path,
    row: &Value,
    persona: &str,
    out: &mut Vec<ReviewFailure>,
) {
    for key in ["prompt_packet", "review_report"] {
        artifact_ref_error(root, row.get(key), persona, key, out);
    }
    let evidence_rows = row
        .get("evidence_paths_checked")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let evidence_paths = evidence_rows
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    if evidence_rows.len() < 3 || evidence_paths.len() != evidence_rows.len() {
        out.push(ReviewFailure::new(
            "validator-execution-provenance",
            "review_round_insufficient_execution_evidence",
            persona,
        ));
    }
}

pub(crate) fn artifact_ref_error(
    root: &Path,
    artifact: Option<&Value>,
    persona: &str,
    key: &str,
    out: &mut Vec<ReviewFailure>,
) {
    let Some(artifact) = artifact else {
        out.push(ReviewFailure::new(
            "validator-execution-provenance",
            "review_round_report_artifact_missing",
            persona,
        ));
        return;
    };
    let rel = artifact.get("path").and_then(Value::as_str).unwrap_or("");
    if crate::package::inventory::package_path_error(root, rel).is_some() {
        out.push(ReviewFailure::new(
            "validator-execution-provenance",
            "review_round_report_artifact_invalid",
            format!("{persona}:{key}"),
        ));
        return;
    }
    let path = root.join(rel);
    let want = artifact.get("digest").and_then(Value::as_str).unwrap_or("");
    let got = digest::file(&path);
    if want == digest::ZERO || got.as_deref() != Ok(want) {
        out.push(ReviewFailure::new(
            "validator-execution-provenance",
            "review_round_report_artifact_mismatch",
            format!("{persona}:{key}"),
        ));
    }
}
