use crate::audit::contract::Failure;
use serde_json::Value;
use std::path::Path;

pub fn check(claim: &Value, root: &Path, out: &mut Vec<Failure>) {
    if crate::claim_semantics::str_field(claim, "claim_ceiling_effect") != "included" {
        return;
    }
    if !product_fitness_applicable(claim) {
        return;
    }
    let receipt_failures = fitness_receipt_failures(claim, root);
    if receipt_failures.is_empty() {
        substitution_checks(claim, true, out);
    } else {
        let has_missing_receipt_failure = receipt_failures
            .iter()
            .any(|failure| failure.error == "product_fitness_quality_in_use_receipt_missing");
        for failure in receipt_failures {
            out.push(failure);
        }
        if !has_missing_receipt_failure {
            out.push(fail(
                "product_fitness_quality_in_use_receipt_missing",
                crate::claim_semantics::str_field(claim, "id"),
            ));
        }
        substitution_checks(claim, false, out);
    }
}

fn product_fitness_applicable(claim: &Value) -> bool {
    let text = crate::claim_semantics::claim::proof::claim_text(claim);
    crate::claim_semantics::product::cohesion::claim_text_product_applicable(claim)
        || crate::claim_semantics::bool_field(claim, "requires_product_cohesion")
        || crate::claim_semantics::str_field(claim, "claim_kind") == "product::cohesion"
        || crate::claim_semantics::str_field(claim, "claim_surface") == "product::cohesion"
        || contains_any(
            &text,
            &[
                "product success",
                "product readiness",
                "customer ready",
                "operator ready",
                "daily driver",
                "release ready",
                "market fit",
                "quality in use",
                "install success",
                "onboarding success",
            ],
        )
}

fn fitness_receipt_failures(claim: &Value, root: &Path) -> Vec<Failure> {
    let mut out = Vec::new();
    let Some(ev) = crate::claim_semantics::evidence(claim)
        .into_iter()
        .find(|ev| {
            crate::claim_semantics::str_field(ev, "kind") == "product::fitness::receipt"
                && crate::claim_semantics::str_field(ev, "surface") == "product::fitness"
        })
    else {
        return vec![fail(
            "product_fitness_quality_in_use_receipt_missing",
            crate::claim_semantics::str_field(claim, "id"),
        )];
    };
    let path = crate::claim_semantics::str_field(ev, "path");
    if crate::package::inventory::package_path_error(root, &path).is_some() {
        return vec![fail("product_fitness_receipt_path_invalid", path)];
    }
    let actual_digest = match crate::digest::file(&root.join(&path)) {
        Ok(digest) => digest,
        Err(_) => return vec![fail("product_fitness_receipt_missing", path)],
    };
    if actual_digest != crate::claim_semantics::str_field(ev, "digest") {
        out.push(fail(
            "product_fitness_receipt_digest_mismatch",
            path.clone(),
        ));
    }
    let receipt = match crate::json_boundary::read_json(&root.join(&path)) {
        Ok(value) => value,
        Err(_) => return vec![fail("product_fitness_receipt_malformed", path)],
    };
    if receipt.pointer("/claim/id").and_then(Value::as_str)
        != Some(crate::claim_semantics::str_field(claim, "id").as_str())
    {
        out.push(fail(
            "product_fitness_receipt_wrong_claim_id",
            crate::claim_semantics::str_field(claim, "id"),
        ));
    }
    repeated_use_text_checks(claim, &receipt, &mut out);
    out.extend(
        crate::audit::product::fitness::receipt_value_failures(root, &receipt)
            .into_iter()
            .map(|error| fail(&error, crate::claim_semantics::str_field(claim, "id"))),
    );
    out
}

fn repeated_use_text_checks(claim: &Value, receipt: &Value, out: &mut Vec<Failure>) {
    let text = crate::claim_semantics::claim::proof::claim_text(claim);
    let repeated_text = text.contains(" daily driver ")
        || text.contains(" repeated use ")
        || text.contains(" continuance ")
        || text.contains(" retention ");
    if !repeated_text {
        return;
    }
    let receipt_repeated = receipt
        .pointer("/claim/repeated_use_claimed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let has_continuance = receipt
        .pointer("/continuance_signal/evidence/path")
        .and_then(Value::as_str)
        .is_some();
    if !receipt_repeated || !has_continuance {
        out.push(fail(
            "product_fitness_daily_driver_overclaim",
            crate::claim_semantics::str_field(claim, "id"),
        ));
    }
}

fn substitution_checks(claim: &Value, has_valid_receipt: bool, out: &mut Vec<Failure>) {
    let text = crate::claim_semantics::claim::proof::claim_text(claim);
    for (needle, error) in [
        (
            "reviewer approved",
            "product_fitness_reviewer_substituted_for_user_evidence",
        ),
        (
            "install success",
            "product_fitness_install_substituted_for_success",
        ),
        (
            "smoke test",
            "product_fitness_smoke_test_substituted_for_success",
        ),
        (
            "tests passed",
            "product_fitness_test_pass_substituted_for_user_value",
        ),
        (
            "package publication",
            "product_fitness_package_publication_substituted_for_product_success",
        ),
        (
            "first use",
            "product_fitness_first_use_substituted_for_success",
        ),
        (
            "feature delivered",
            "product_fitness_output_substituted_for_outcome",
        ),
    ] {
        if text.contains(needle) && !has_valid_receipt {
            out.push(fail(error, crate::claim_semantics::str_field(claim, "id")));
        }
    }
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn fail(error: &str, detail: impl Into<String>) -> Failure {
    Failure::new("product-fitness-proof", error, detail)
}
