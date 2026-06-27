pub(crate) mod criteria;

use crate::review::round::ReviewFailure;
use serde_json::Value;
use std::path::Path;

const OWNER_PERSONA: &str = "product_simplicity_falsifier";
const OWNER_AGENT: &str = "harness_product_simplicity_falsifier";

pub(crate) fn disposition_errors(
    root: &Path,
    row: &Value,
    receipt: &Value,
    persona: &str,
    out: &mut Vec<ReviewFailure>,
) {
    if persona != OWNER_PERSONA || !criteria::product_impacting_claims(receipt) {
        return;
    }
    scalar_binding_errors(row, persona, out);
    let Some(disposition) = row.get("product_fitness_disposition") else {
        push(
            out,
            "review_round_product_fitness_disposition_missing",
            persona,
        );
        return;
    };
    owner_errors(row, disposition, persona, out);
    claim_binding_errors(row, disposition, receipt, persona, out);
    substitution_errors(row, disposition, persona, out);
    if receipt_stale(root, row, disposition) {
        push(
            out,
            "review_round_product_fitness_disposition_stale",
            persona,
        );
    }
}

fn scalar_binding_errors(row: &Value, persona: &str, out: &mut Vec<ReviewFailure>) {
    if row.get("product_fitness_required").and_then(Value::as_bool) != Some(true) {
        push(
            out,
            "review_round_product_fitness_required_missing",
            persona,
        );
    }
    if row.get("product_fitness_owner").and_then(Value::as_str) != Some(OWNER_PERSONA) {
        push(out, "review_round_product_fitness_owner_missing", persona);
    }
    if row
        .get("product_fitness_receipt_digest")
        .and_then(Value::as_str)
        .unwrap_or("")
        .is_empty()
    {
        push(
            out,
            "review_round_product_fitness_receipt_binding_missing",
            persona,
        );
    }
}

fn owner_errors(row: &Value, disposition: &Value, persona: &str, out: &mut Vec<ReviewFailure>) {
    if disposition.get("owner_persona").and_then(Value::as_str) != Some(OWNER_PERSONA)
        || disposition.get("owner_agent_type").and_then(Value::as_str) != Some(OWNER_AGENT)
        || row.get("agent_type").and_then(Value::as_str) != Some(OWNER_AGENT)
        || disposition
            .get("applies_to_product_impacting_claims")
            .and_then(Value::as_bool)
            != Some(true)
    {
        push(
            out,
            "review_round_product_fitness_disposition_unowned",
            persona,
        );
    }
}

fn claim_binding_errors(
    row: &Value,
    disposition: &Value,
    receipt: &Value,
    persona: &str,
    out: &mut Vec<ReviewFailure>,
) {
    let claims = criteria::string_set(row, "product_fitness_claim_ids");
    let disposition_claims = criteria::string_set(disposition, "claim_ids_reviewed");
    if claims.is_empty() || disposition_claims.is_empty() || claims != disposition_claims {
        push(
            out,
            "review_round_product_fitness_claim_binding_missing",
            persona,
        );
    }
    let product_claims = criteria::product_claim_ids(receipt);
    if !product_claims.is_empty() && !product_claims.is_subset(&claims) {
        push(
            out,
            "review_round_product_fitness_claim_binding_missing",
            persona,
        );
    }
}

fn substitution_errors(
    row: &Value,
    disposition: &Value,
    persona: &str,
    out: &mut Vec<ReviewFailure>,
) {
    let reviewed = criteria::string_set(row, "substitution_rejections_reviewed");
    if !criteria::required_substitutions().is_subset(&reviewed)
        || disposition
            .get("generic_product_simplicity_approval_only")
            .and_then(Value::as_bool)
            != Some(false)
        || disposition
            .get("substitution_rejection")
            .and_then(Value::as_bool)
            != Some(true)
        || !criteria::dimensions_complete(disposition)
    {
        push(
            out,
            "review_round_product_fitness_generic_approval_substitution",
            persona,
        );
    }
}

fn receipt_stale(root: &Path, row: &Value, disposition: &Value) -> bool {
    let artifact = &disposition["receipt"];
    let rel = artifact.get("path").and_then(Value::as_str).unwrap_or("");
    if crate::package::inventory::package_path_error(root, rel).is_some() {
        return true;
    }
    let path = root.join(rel);
    let want = artifact.get("digest").and_then(Value::as_str).unwrap_or("");
    if row
        .get("product_fitness_receipt_digest")
        .and_then(Value::as_str)
        != Some(want)
    {
        return true;
    }
    if crate::digest::file(&path).map_or(true, |actual| actual != want) {
        return true;
    }
    let Ok(value) = crate::json_boundary::read_json(&path) else {
        return true;
    };
    value.get("generated_at").and_then(Value::as_str)
        != disposition
            .get("receipt_generated_at")
            .and_then(Value::as_str)
}

fn push(out: &mut Vec<ReviewFailure>, error: &str, detail: &str) {
    out.push(ReviewFailure::new(
        "validator-execution-provenance",
        error,
        detail,
    ));
}
