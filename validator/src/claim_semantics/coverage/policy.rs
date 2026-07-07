use crate::audit::contract::Failure;
use crate::claim_semantics::coverage::receipt::authority::DigestCache;
use crate::claim_semantics::{evidence, str_field};
use serde_json::Value;
use std::path::Path;

#[cfg(test)]
pub fn check(claim: &Value, root: &Path, out: &mut Vec<Failure>) {
    let mut cache = DigestCache::default();
    check_with_cache(claim, root, out, &mut cache);
}

pub(crate) fn check_with_cache(
    claim: &Value,
    root: &Path,
    out: &mut Vec<Failure>,
    cache: &mut DigestCache,
) {
    let text = crate::claim::text::normalized_text(&[
        &str_field(claim, "title"),
        &str_field(claim, "description"),
    ]);
    if !coverage_relevant(&text) {
        return;
    }
    let receipts = coverage_evidence(claim);
    if receipts.is_empty() {
        let error = substitution_error(&text).unwrap_or("coverage_receipt_missing");
        out.push(Failure::new(
            "coverage-proof-policy",
            error,
            str_field(claim, "id"),
        ));
        return;
    }
    for ev in receipts {
        check_receipt(claim, ev, root, &text, out, cache);
    }
}

fn coverage_relevant(text: &str) -> bool {
    text.contains("coverage")
        || text.contains("covered")
        || text.contains("uncovered")
        || (mentions_tests(text) && completion_claim(text))
        || (substitution_error(text).is_some() && completion_claim(text))
}

fn mentions_tests(text: &str) -> bool {
    [
        "test pass",
        "tests pass",
        "smoke test",
        "fixture test",
        "mock",
        "examples ran",
        "generated examples passed",
    ]
    .iter()
    .any(|term| text.contains(term))
}

fn completion_claim(text: &str) -> bool {
    [
        "complete",
        "done",
        "ready",
        "production-ready",
        "release",
        "release-ready",
        "release advancement",
        "material sign-off",
    ]
    .iter()
    .any(|term| text.contains(term))
}

fn substitution_error(text: &str) -> Option<&'static str> {
    if ["reviewer approved", "reviewer signoff", "reviewer sign-off"]
        .iter()
        .any(|term| text.contains(term))
    {
        return Some("coverage_reviewer_signoff_substitution");
    }
    if ["ci passed", "all checks passed", "checks passed"]
        .iter()
        .any(|term| text.contains(term))
    {
        return Some("coverage_check_pass_substitution");
    }
    if mentions_tests(text) || text.contains("target fixture passed") {
        return Some("coverage_test_pass_substitution");
    }
    None
}

fn coverage_evidence(claim: &Value) -> Vec<&Value> {
    evidence(claim)
        .into_iter()
        .filter(|ev| str_field(ev, "kind") == "coverage_receipt")
        .collect()
}

fn check_receipt(
    claim: &Value,
    ev: &Value,
    root: &Path,
    text: &str,
    out: &mut Vec<Failure>,
    cache: &mut DigestCache,
) {
    let Ok(path) = crate::package::inventory::resolve(root, &str_field(ev, "path")) else {
        out.push(Failure::new(
            "coverage-proof-policy",
            "coverage_receipt_missing",
            str_field(ev, "id"),
        ));
        return;
    };
    let Ok(receipt) = crate::json_boundary::read_json(&path) else {
        out.push(Failure::new(
            "coverage-proof-policy",
            "coverage_receipt_malformed",
            str_field(ev, "id"),
        ));
        return;
    };
    evidence_freshness_checks(ev, out);
    crate::claim_semantics::coverage::receipt::rules::shape(claim, &receipt, ev, out);
    crate::claim_semantics::coverage::receipt::authority::check_with_cache(
        &receipt, root, out, cache,
    );
    crate::claim_semantics::coverage::receipt::rules::percent(&receipt, text, out);
    crate::claim_semantics::coverage::receipt::rules::dimensions(&receipt, text, out);
    crate::claim_semantics::coverage::receipt::exclusions::check(&receipt, out);
}

fn evidence_freshness_checks(ev: &Value, out: &mut Vec<Failure>) {
    if ev
        .pointer("/freshness/verdict")
        .and_then(Value::as_str)
        .is_some_and(|verdict| verdict != "current")
    {
        out.push(Failure::new(
            "coverage-proof-policy",
            "coverage_receipt_stale",
            str_field(ev, "id"),
        ));
    }
}
