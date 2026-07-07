use crate::{digest, review::round::ReviewFailure, review::round::anchor::values::AnchorValues};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

pub(crate) fn report_errors(
    root: &Path,
    receipt: &Value,
    row: &Value,
    anchors: &AnchorValues,
    persona: &str,
    out: &mut Vec<ReviewFailure>,
) {
    let Some(path) = report_path(root, row, persona, out) else {
        return;
    };
    let text = report_text(&path);
    let _ = (row, anchors);
    report_text_errors(&text, persona, out);
    evidence_artifact_errors(root, receipt, row, persona, out);
}

fn report_path(
    root: &Path,
    row: &Value,
    persona: &str,
    out: &mut Vec<ReviewFailure>,
) -> Option<PathBuf> {
    let rel = row
        .pointer("/review_report/path")
        .and_then(Value::as_str)
        .unwrap_or("");
    match crate::package::inventory::resolve(root, rel) {
        Ok(path) => Some(path),
        Err(_) => {
            out.push(failure("review_round_report_path_invalid", persona));
            None
        }
    }
}

fn report_text(path: &Path) -> String {
    crate::digest::read_file_bytes(path)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .unwrap_or_default()
}

fn report_text_errors(text: &str, persona: &str, out: &mut Vec<ReviewFailure>) {
    if text.contains(concat!("/", "Users/")) {
        out.push(failure("review_round_private_path_leak", persona));
    }
    let lower = text.to_ascii_lowercase();
    let has_disclaimer = lower.contains("narrative attachment only")
        && lower.contains("typed review-round receipt")
        && lower.contains("validated by rust");
    let scan_text = lower.replace(
        "this fixture report is a narrative attachment only. it is not an authority for verdicts, blockers, counterexample coverage, proof anchors, claim ceilings, or next-stage routing. those obligations live in the typed review-round receipt and are validated by rust.",
        "",
    );
    for phrase in [
        "claim ceiling",
        "material blocker",
        "counterexample",
        "proof anchor",
        "recommended next stage",
        "sign off",
        "sign-off",
    ] {
        if scan_text.contains(phrase) {
            out.push(failure("review_round_report_authority_prose", persona));
            return;
        }
    }
    if !has_disclaimer {
        out.push(failure("review_round_report_missing_disclaimer", persona));
    }
}

fn evidence_artifact_errors(
    root: &Path,
    receipt: &Value,
    row: &Value,
    persona: &str,
    out: &mut Vec<ReviewFailure>,
) {
    let rows = row
        .get("evidence_artifacts_checked")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if rows.len() < 6 {
        out.push(failure("review_round_hollow_evidence_artifacts", persona));
        return;
    }
    let mut paths = BTreeSet::new();
    for artifact in &rows {
        crate::review::round::artifacts::artifact_ref_error(
            root,
            Some(artifact),
            persona,
            "evidence_artifacts_checked",
            out,
        );
        if let Some(path) = artifact.get("path").and_then(Value::as_str) {
            paths.insert(path.to_string());
        }
    }
    if paths.len() != rows.len() {
        out.push(failure("review_round_hollow_evidence_artifacts", persona));
        return;
    }
    for path in required_paths(receipt, row, persona) {
        if path.is_empty() || !paths.contains(&path) || digest::file(&root.join(&path)).is_err() {
            out.push(failure("review_round_hollow_evidence_artifacts", persona));
            return;
        }
    }
}

fn required_paths(receipt: &Value, row: &Value, persona: &str) -> Vec<String> {
    let mut paths = vec![
        pointer(receipt, "/validator_receipt/path"),
        pointer(receipt, "/review_target/path"),
        pointer(receipt, "/archive/path"),
        string(row, "persona_prompt_path"),
        string(row, "custom_agent_path"),
    ];
    if let Some(spec) = crate::review::round::config::persona_spec(persona) {
        paths.push(spec.focus_path.to_string());
    }
    paths
}

fn pointer(value: &Value, ptr: &str) -> String {
    value
        .pointer(ptr)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn string(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn failure(code: &str, detail: impl Into<String>) -> ReviewFailure {
    ReviewFailure::new("validator-execution-provenance", code, detail)
}
