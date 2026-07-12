use crate::review::round::{
    ReviewFailure,
    anchor::{
        policy::{self, AnchorSource},
        values::AnchorValues,
    },
    config::{
        self, MATERIALITY_ADVISORY as ADVISORY, MATERIALITY_BLOCKED as BLOCKED,
        MATERIALITY_DELTA as DELTA, MATERIALITY_FULL as FULL, MaterialityAuthority,
        REVIEW_ROLES as ROLES,
    },
};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

mod registry;
use registry::registry_contents_valid;

pub(crate) fn review_round_errors(
    root: &Path,
    receipt: &Value,
    anchors: &AnchorValues,
    out: &mut Vec<ReviewFailure>,
) {
    let gate = &receipt["materiality_gate"];
    for error in failures(gate, Some(root), Some(receipt), Some(anchors)) {
        out.push(ReviewFailure::new(
            "material-review-scope-gate",
            &error,
            "materiality_gate",
        ));
    }
    if receipt.get("review_stage").and_then(Value::as_str) == Some("falsification") {
        if text(gate, "decision") != FULL {
            out.push(failure("material_falsification_requires_full_scope"));
        }
        if gate["output_may_be_used_for_material_signoff"].as_bool() != Some(false) {
            out.push(failure("materiality_reviewer_cannot_authorize_signoff"));
        }
    }
}

pub(crate) fn fixture_failures(root: &Path) -> Vec<String> {
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
            Ok(value) => failures(&value, Some(root), None, None)
                .into_iter()
                .map(move |error| format!("{}: {error}", path.display()))
                .collect::<Vec<_>>(),
            Err(err) => vec![format!("{}: {err}", path.display())],
        })
        .collect()
}

pub(crate) fn value_failures(value: &Value) -> Vec<String> {
    failures(value, None, None, None)
}

fn failures(
    value: &Value,
    root: Option<&Path>,
    receipt: Option<&Value>,
    bound_anchors: Option<&AnchorValues>,
) -> Vec<String> {
    let mut out = Vec::new();
    if text(value, "schema") != "harness-ultragoal.review-materiality-gate.v2" {
        out.push("materiality_schema_invalid".into());
    }
    if text(value, "decision_authority") != "deterministic_rust"
        || text(value, "reviewer_output_authority") != "falsification_only_cannot_raise_claims"
    {
        out.push("materiality_authority_invalid".into());
    }
    let source = receipt
        .map(policy::receipt_source)
        .unwrap_or(Some(AnchorSource::StaticFixture));
    let anchors_ok = identity_errors(value, root, receipt, source, bound_anchors, &mut out);
    let authority = match source {
        Some(AnchorSource::StaticFixture) => MaterialityAuthority::StaticFixture,
        Some(AnchorSource::Live) => MaterialityAuthority::LiveObservationUnavailable,
        None => MaterialityAuthority::CanonicalSourceUnavailable,
    };
    let derived = config::derive_materiality(value, anchors_ok, authority);
    out.extend(derived.errors);
    let decision = text(value, "decision");
    if ![FULL, DELTA, ADVISORY, BLOCKED].contains(&decision) {
        out.push("materiality_decision_invalid".into());
    } else if decision != derived.decision {
        out.push("materiality_decision_derivation_mismatch".into());
    }
    claim_ceiling_errors(value, &mut out);
    match decision {
        FULL => full_errors(value, &mut out),
        DELTA => delta_errors(value, &mut out),
        ADVISORY => advisory_errors(value, &mut out),
        BLOCKED => blocked_errors(value, &mut out),
        _ => {}
    }
    out
}

fn identity_errors(
    value: &Value,
    root: Option<&Path>,
    receipt: Option<&Value>,
    source: Option<AnchorSource>,
    bound_anchors: Option<&AnchorValues>,
    out: &mut Vec<String>,
) -> bool {
    let expected_source = source.unwrap_or(AnchorSource::StaticFixture);
    let expected = policy::expected_anchors(expected_source);
    let anchors = match bound_anchors {
        Some(bound) => crate::review::round::anchor::refs::verify_loaded(
            value,
            "anchors_checked",
            expected,
            &bound.materiality_anchors,
        ),
        None => {
            crate::review::round::anchor::refs::verify(value, "anchors_checked", expected, root)
        }
    };
    let registry = crate::review::round::anchor::refs::verify(
        value,
        "reviewer_registry_evidence",
        policy::expected_registry(expected_source),
        root,
    );
    if anchors.is_none() {
        if value["anchors_checked"]
            .as_array()
            .is_none_or(Vec::is_empty)
        {
            out.push("materiality_anchor_evidence_missing".into());
        }
        out.push("materiality_anchor_identity_invalid".into());
    }
    if registry.is_none() {
        if value["reviewer_registry_evidence"]
            .as_array()
            .is_none_or(Vec::is_empty)
        {
            out.push("materiality_reviewer_registry_evidence_missing".into());
        }
        out.push("materiality_reviewer_registry_identity_invalid".into());
    }
    let contents_ok = root.is_some()
        && anchors
            .as_ref()
            .is_some_and(|rows| anchor_contents_valid(rows))
        && registry
            .as_ref()
            .is_some_and(|rows| registry_contents_valid(&rows[0]));
    if !contents_ok {
        out.push("materiality_gate_evidence_unverified".into());
    }
    let linked = receipt.is_none_or(|receipt| {
        let paths = [
            text_at(receipt, "/validator_receipt/path"),
            text_at(receipt, "/review_target/path"),
            text_at(receipt, "/archive/path"),
        ];
        anchors.as_ref().is_some_and(|rows| {
            paths
                .iter()
                .all(|path| rows.iter().any(|row| row.path == *path))
        }) && registry.as_ref().is_some_and(|rows| {
            rows[0].path == text_at(receipt, "/live_registry_exposure/path")
                && rows[0].digest == text_at(receipt, "/live_registry_exposure/digest")
        })
    });
    if !linked {
        out.push("materiality_evidence_substitution".into());
    }
    anchors.is_some() && registry.is_some() && contents_ok && linked
}

fn anchor_contents_valid(rows: &[crate::review::round::anchor::refs::VerifiedRef]) -> bool {
    rows.iter().all(|row| {
        row.value
            .as_ref()
            .is_some_and(|value| value["status"] == "pass")
    })
}

fn claim_ceiling_errors(value: &Value, out: &mut Vec<String>) {
    let Some(ceiling) = value.get("claim_ceiling") else {
        out.push("materiality_claim_ceiling_missing".into());
        return;
    };
    if ["supported", "unsupported", "blocked"]
        .iter()
        .any(|key| !ceiling.get(key).is_some_and(Value::is_array))
    {
        out.push("materiality_claim_ceiling_missing".into());
    }
}

fn full_errors(value: &Value, out: &mut Vec<String>) {
    let reviewers = strings(value, "reviewers_required");
    if ROLES.iter().any(|spec| !reviewers.contains(spec.role_name))
        || reviewers.len() != ROLES.len()
    {
        out.push("materiality_full_scope_reviewer_set_missing".into());
    }
    no_signoff(value, out, "materiality_reviewer_cannot_authorize_signoff");
}

fn delta_errors(value: &Value, out: &mut Vec<String>) {
    no_signoff(value, out, "materiality_delta_used_for_signoff");
    let count = strings(value, "reviewers_required").len();
    if count > 1 {
        out.push("materiality_delta_reviewer_scope_too_broad".into());
    } else if count == 0 {
        out.push("materiality_delta_reviewer_scope_invalid".into());
    }
}

fn advisory_errors(value: &Value, out: &mut Vec<String>) {
    no_signoff(value, out, "materiality_advisory_used_for_signoff");
    if !strings(value, "reviewers_required").is_empty()
        || value
            .pointer("/claim_ceiling/supported")
            .and_then(Value::as_array)
            .map_or(0, Vec::len)
            != 0
    {
        out.push("materiality_advisory_overclaims_support".into());
    }
}

fn blocked_errors(value: &Value, out: &mut Vec<String>) {
    if !strings(value, "reviewers_required").is_empty() {
        out.push("materiality_blocked_launches_reviewers".into());
    }
    if strings(value, "validator_repairs_recommended").is_empty() {
        out.push("materiality_blocked_without_repair".into());
    }
}

fn no_signoff(value: &Value, out: &mut Vec<String>, error: &str) {
    if value["output_may_be_used_for_material_signoff"].as_bool() != Some(false) {
        out.push(error.into());
    }
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

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}

fn text_at<'a>(value: &'a Value, pointer: &str) -> &'a str {
    value.pointer(pointer).and_then(Value::as_str).unwrap_or("")
}

fn failure(error: &str) -> ReviewFailure {
    ReviewFailure::new("material-review-scope-gate", error, "materiality_gate")
}
