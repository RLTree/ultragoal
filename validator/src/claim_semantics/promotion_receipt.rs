use crate::audit::contract::Failure;
use crate::claim_semantics::{evidence, good_status, str_field};
use crate::{json_boundary, schema_catalog};
use serde_json::Value;
use std::path::Path;

pub fn check(claim: &Value, root: &Path, out: &mut Vec<Failure>) {
    if !promotion_claim_included(claim) {
        return;
    }
    let receipts = evidence(claim)
        .into_iter()
        .filter(|ev| str_field(ev, "kind") == "promotion_receipt")
        .collect::<Vec<_>>();
    if receipts.is_empty() {
        push(out, "promotion_receipt_required", claim, "missing");
        return;
    }
    for ev in receipts {
        if receipt_invalid(claim, ev, root) {
            push(
                out,
                "promotion_receipt_invalid",
                claim,
                &str_field(ev, "id"),
            );
        }
    }
}

fn promotion_claim_included(claim: &Value) -> bool {
    if str_field(claim, "claim_ceiling_effect") != "included"
        && !good_status(&str_field(claim, "status"))
    {
        return false;
    }
    let semantic = &claim["semantic_text_classification"];
    if [
        "install_visibility_claim",
        "publication_claim",
        "distribution_claim",
    ]
    .iter()
    .any(|key| semantic.get(*key).and_then(Value::as_bool) == Some(true))
    {
        return true;
    }
    let text = crate::claim_semantics::claim::proof::claim_text(claim);
    let tokens = crate::claim::text::tokens(&text);
    crate::claim::language::install_visibility_claim(&text, &tokens)
        || crate::claim::language::publication_claim(&text, &tokens)
}

fn receipt_invalid(claim: &Value, ev: &Value, root: &Path) -> bool {
    if str_field(ev, "surface") != "external" {
        return true;
    }
    if crate::package::artifact::refs::validate_object(root, ev, "promotion receipt evidence")
        .is_err()
    {
        return true;
    }
    let path = str_field(ev, "path");
    let Ok(receipt) = json_boundary::read_json(&root.join(path)) else {
        return true;
    };
    let store = schema_catalog::load(root);
    if !schema_catalog::schema_errors(&store, "promotion-receipt.schema.json", &receipt).is_empty()
    {
        return true;
    }
    if str_field(&receipt, "claim_id") != str_field(claim, "id") {
        return true;
    }
    if !target_matches_claim(claim, &receipt) {
        return true;
    }
    !anchors_internally_joined(&receipt)
}

pub(crate) fn target_matches_claim(claim: &Value, receipt: &Value) -> bool {
    let target = str_field(receipt, "promotion_target");
    let semantic = &claim["semantic_text_classification"];
    match target.as_str() {
        "install_visibility" => {
            semantic
                .get("install_visibility_claim")
                .and_then(Value::as_bool)
                == Some(true)
        }
        "publication" => semantic.get("publication_claim").and_then(Value::as_bool) == Some(true),
        "upload_distribution" => {
            semantic.get("distribution_claim").and_then(Value::as_bool) == Some(true)
        }
        _ => false,
    }
}

fn anchors_internally_joined(receipt: &Value) -> bool {
    let validator_run = pointer(receipt, "/validator_receipt/run_id");
    let review_target = pointer(receipt, "/review_target/review_target_digest");
    let archive_digest = pointer(receipt, "/candidate_archive/digest");
    let package_digest = pointer(receipt, "/validator_receipt/package_digest");
    let review_package = pointer(receipt, "/review_target/package_digest");
    let archive_package = pointer(receipt, "/candidate_archive/package_digest");
    !(validator_run.is_empty()
        || review_target.is_empty()
        || archive_digest.is_empty()
        || package_digest != review_package
        || package_digest != archive_package
        || validator_run != pointer(receipt, "/sign_off_review/validator_run_id")
        || review_target != pointer(receipt, "/sign_off_review/review_target_digest")
        || archive_digest != pointer(receipt, "/sign_off_review/archive_digest"))
}

fn pointer(value: &Value, ptr: &str) -> String {
    value
        .pointer(ptr)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn push(out: &mut Vec<Failure>, error: &str, claim: &Value, detail: &str) {
    out.push(Failure::new(
        "claim-status-ceiling",
        error,
        format!("{}:{detail}", str_field(claim, "id")),
    ));
}
