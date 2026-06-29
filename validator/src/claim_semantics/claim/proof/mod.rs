pub(crate) mod text;
use crate::audit::contract::Failure;
use crate::claim_semantics::coverage::receipt::authority::DigestCache;
use crate::claim_semantics::{array_strings, bool_field, evidence, good_status, str_field};
use crate::claim_semantics::{
    claim::evidence as claim_evidence, coverage::policy as coverage_policy, dogfood_receipt,
    product::cohesion, product::fitness, promotion_receipt,
    semantic::receipt::policy as semantic_policy,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) fn check_claim_with_cache(
    claim: &Value,
    rows: &BTreeMap<String, &Value>,
    cm: &Value,
    ready: &Value,
    root: &Path,
    out: &mut Vec<Failure>,
    coverage_cache: &mut DigestCache,
) {
    ceiling_checks(claim, rows, out);
    observability_checks(claim, out);
    text::text_surface_checks(claim, out);
    out.extend(cohesion::product_cohesion_failures(claim));
    fitness::check(claim, root, out);
    promotion_receipt::check(claim, root, out);
    dogfood_receipt::check(claim, root, out);
    coverage_policy::check_with_cache(claim, root, out, coverage_cache);
    if bool_field(claim, "requires_goal_binding") && !goal_binding_satisfied(cm) {
        out.push(Failure::new(
            "goal-binding-receipt-match",
            "goal_bound_claim_without_bound_goal",
            str_field(claim, "id"),
        ));
    }
    claim_evidence::evidence_checks(claim, cm, ready, root, out);
    if str_field(claim, "claim_kind") == "feature_completion"
        && str_field(claim, "claim_ceiling_effect") == "included"
    {
        claim_evidence::live_e2e_check(claim, ready, root, out);
    }
    if str_field(claim, "claim_kind") == "connector_capability"
        && str_field(claim, "claim_ceiling_effect") == "included"
    {
        out.push(Failure::new(
            "connector-capability-discovery",
            "connector_capability_without_discovery",
            str_field(claim, "id"),
        ));
    }
    semantic_policy::check_semantic_receipts(claim, ready, root, out);
}

fn goal_binding_satisfied(cm: &Value) -> bool {
    let binding = &cm["goal_binding"];
    let receipt = &binding["get_goal_receipt"];
    str_field(binding, "status") == "bound"
        && str_field(receipt, "workspace") == str_field(cm, "root")
        && str_field(receipt, "observed_goal_id") == str_field(binding, "goal_id")
        && str_field(receipt, "observed_objective") == str_field(binding, "objective")
        && str_field(receipt, "observed_contract_path") == str_field(binding, "contract_path")
}

fn ceiling_checks(claim: &Value, rows: &BTreeMap<String, &Value>, out: &mut Vec<Failure>) {
    let row_id = str_field(claim, "backlog_row_id");
    if !good_status(&str_field(claim, "status"))
        && (row_id.is_empty() || !rows.contains_key(&row_id))
    {
        out.push(Failure::new(
            "claim-status-ceiling",
            "withheld_claim_missing_backlog_row",
            str_field(claim, "id"),
        ));
    }
}

fn observability_checks(claim: &Value, out: &mut Vec<Failure>) {
    let requires_obs = bool_field(claim, "requires_observability")
        || str_field(claim, "claim_surface") == "observability"
        || array_strings(claim, "allowed_evidence_surfaces")
            .iter()
            .any(|s| s == "observability");
    let has_obs = evidence(claim).iter().any(|ev| {
        str_field(ev, "kind") == "observability_receipt"
            && str_field(ev, "surface") == "observability"
    });
    if requires_obs && str_field(claim, "claim_ceiling_effect") == "included" && !has_obs {
        out.push(Failure::new(
            "observability-claim-proof",
            "observability_claim_without_receipt",
            str_field(claim, "id"),
        ));
    }
}

pub(crate) fn claim_text(claim: &Value) -> String {
    crate::claim::text::normalized_text(&[
        &str_field(claim, "title"),
        &str_field(claim, "description"),
    ])
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn goal_binding_and_observability_receipt_match_exact_authority() {
        let cm = json!({
            "root":"workspace",
            "goal_binding":{
                "status":"bound",
                "goal_id":"goal",
                "objective":"objective",
                "contract_path":"contract.md",
                "get_goal_receipt":{
                    "workspace":"workspace",
                    "observed_goal_id":"goal",
                    "observed_objective":"objective",
                    "observed_contract_path":"contract.md"
                }
            }
        });
        assert!(super::goal_binding_satisfied(&cm));
        let mut out = Vec::new();
        super::observability_checks(
            &json!({
                "id":"OBS",
                "claim_ceiling_effect":"included",
                "requires_observability":true,
                "evidence":[{"kind":"observability_receipt","surface":"observability"}]
            }),
            &mut out,
        );
        assert!(out.is_empty(), "{out:?}");
    }
}
