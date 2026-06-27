use crate::{review::round::ReviewFailure, review::round::anchor::values::AnchorValues};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

const SUPPORTED: &[&str] = &[
    "package_static_fixture_proof",
    "detached_review_target_archive_identity",
];
const UNSUPPORTED: &[&str] = &[
    "plugins_ui_visibility",
    "install_button_success",
    "workspace_public_marketplace_publication",
    "real_multilane_dogfood",
    "production_readiness",
    "external_product_ux_improvement",
];

pub(crate) fn row_authority_errors(
    root: &Path,
    receipt: &Value,
    anchors: &AnchorValues,
    row: &Value,
    persona: &str,
    out: &mut Vec<ReviewFailure>,
) {
    counterexample_errors(root, row, persona, out);
    proof_anchor_errors(root, anchors, row, persona, out);
    next_repair_errors(root, row, persona, out);
    claim_ceiling_errors(receipt, row, persona, out);
}

fn counterexample_errors(root: &Path, row: &Value, persona: &str, out: &mut Vec<ReviewFailure>) {
    let probes = row
        .get("counterexamples_attempted")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if probes.len() < 3 {
        out.push(failure("review_round_counterexamples_missing", persona));
    }
    let mut keys = BTreeSet::new();
    for probe in probes {
        let key = format!(
            "{}|{}",
            probe.get("probe_id").and_then(Value::as_str).unwrap_or(""),
            probe
                .get("counterexample")
                .and_then(Value::as_str)
                .unwrap_or("")
        );
        if !keys.insert(key) {
            out.push(failure("review_round_counterexamples_missing", persona));
        }
        crate::review::round::artifacts::artifact_ref_error(
            root,
            probe.get("evidence"),
            persona,
            "counterexamples_attempted",
            out,
        );
    }
}

fn proof_anchor_errors(
    root: &Path,
    anchors: &AnchorValues,
    row: &Value,
    persona: &str,
    out: &mut Vec<ReviewFailure>,
) {
    let rows = row
        .get("proof_anchors_checked")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let checked_paths = rows
        .iter()
        .filter_map(|row| row.get("path").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    let Some(spec) = crate::review::round::config::persona_spec(persona) else {
        return;
    };
    let external_anchors = [
        (&anchors.validator_path, &anchors.validator_digest),
        (&anchors.review_target_path, &anchors.review_target_digest),
        (&anchors.archive_path, &anchors.archive_digest),
    ];
    for artifact in &rows {
        if !external_anchor_matches(artifact, &external_anchors) {
            crate::review::round::artifacts::artifact_ref_error(
                root,
                Some(artifact),
                persona,
                "proof_anchors_checked",
                out,
            );
        }
    }
    let required_anchors = [
        anchors.validator_path.as_str(),
        anchors.review_target_path.as_str(),
        anchors.archive_path.as_str(),
        spec.prompt_path,
        spec.custom_path,
        spec.focus_path,
    ];
    for required in required_anchors {
        if !checked_paths.contains(required) {
            out.push(failure("review_round_report_too_shallow", persona));
            return;
        }
    }
}

fn external_anchor_matches(artifact: &Value, anchors: &[(&String, &String); 3]) -> bool {
    let Some(path) = artifact.get("path").and_then(Value::as_str) else {
        return false;
    };
    let Some(digest) = artifact.get("digest").and_then(Value::as_str) else {
        return false;
    };
    anchors.iter().any(|(want_path, want_digest)| {
        path == want_path.as_str() && digest == want_digest.as_str()
    })
}

fn next_repair_errors(root: &Path, row: &Value, persona: &str, out: &mut Vec<ReviewFailure>) {
    let rows = row
        .get("required_next_repairs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if rows.is_empty() {
        out.push(failure("review_round_report_too_shallow", persona));
    }
    for repair in rows {
        crate::review::round::artifacts::artifact_ref_error(
            root,
            repair.get("evidence"),
            persona,
            "required_next_repairs",
            out,
        );
    }
}

fn claim_ceiling_errors(receipt: &Value, row: &Value, persona: &str, out: &mut Vec<ReviewFailure>) {
    let Some(assessment) = row.get("claim_ceiling_assessment") else {
        out.push(failure("review_round_claim_ceiling_missing", persona));
        return;
    };
    let top_supported = claim_ids(&receipt["claim_ceiling"], "supported");
    let top_unsupported = claim_ids(&receipt["claim_ceiling"], "unsupported");
    let row_supported = claim_ids(assessment, "supported");
    let row_unsupported = claim_ids(assessment, "unsupported");
    if row_supported.is_empty() || row_unsupported.is_empty() {
        out.push(failure("review_round_claim_ceiling_missing", persona));
        return;
    }
    if has_duplicates(assessment, "supported") || has_duplicates(assessment, "unsupported") {
        out.push(failure("review_round_claim_ceiling_duplicate", persona));
        return;
    }
    if unsupported_claim_in_supported(&top_supported)
        || unsupported_claim_in_supported(&row_supported)
        || !same_set(&top_supported, SUPPORTED)
    {
        out.push(failure("review_round_claim_ceiling_overclaim", persona));
        return;
    }
    if !same_set(&top_unsupported, UNSUPPORTED) {
        out.push(failure("review_round_claim_ceiling_missing", persona));
        return;
    }
    if !same_values(&top_supported, &row_supported)
        || !same_values(&top_unsupported, &row_unsupported)
    {
        out.push(failure("review_round_claim_ceiling_mismatch", persona));
    }
}

fn claim_ids<'a>(value: &'a Value, key: &str) -> BTreeSet<&'a str> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| row.get("claim_id").and_then(Value::as_str))
        .collect()
}

fn has_duplicates(value: &Value, key: &str) -> bool {
    let values = value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| row.get("claim_id").and_then(Value::as_str))
        .map(normalize)
        .collect::<Vec<_>>();
    values.len() != values.iter().collect::<BTreeSet<_>>().len()
}

fn same_values(left: &BTreeSet<&str>, right: &BTreeSet<&str>) -> bool {
    left == right
}

fn same_set(values: &BTreeSet<&str>, expected: &[&str]) -> bool {
    values == &expected.iter().copied().collect::<BTreeSet<_>>()
}

fn unsupported_claim_in_supported(values: &BTreeSet<&str>) -> bool {
    values
        .iter()
        .filter(|value| !SUPPORTED.contains(value))
        .any(|value| {
            let normalized = crate::claim::text::normalized_text(&[value]);
            let tokens = crate::claim::text::tokens(&normalized);
            crate::claim::language::install_visibility_claim(&normalized, &tokens)
                || crate::claim::language::publication_claim(&normalized, &tokens)
                || crate::claim::language::dogfood_claim(&normalized, &tokens)
                || crate::claim::language::external_product_claim(&normalized, &tokens)
                || crate::claim::language::live_runtime_claim(&normalized, &tokens)
        })
}

fn normalize(text: &str) -> String {
    text.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect()
}

fn failure(code: &str, detail: impl Into<String>) -> ReviewFailure {
    ReviewFailure::new("validator-execution-provenance", code, detail)
}
