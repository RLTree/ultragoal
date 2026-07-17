pub(crate) mod authority_reconciliation;
pub(crate) mod automation_tick;
pub(crate) mod backlog_policy;
pub(crate) mod claim;
pub(crate) mod coverage;
pub(crate) mod dogfood_receipt;
#[cfg(test)]
mod json_patch;
pub(crate) mod lane;
pub(crate) mod plugin_policy;
pub(crate) mod product;
pub(crate) mod promotion_receipt;
pub(crate) mod ready;
mod retired_reviewer_policy;
pub(crate) mod semantic;

use crate::audit::contract::Failure;
use crate::claim_semantics::coverage::receipt::authority::DigestCache;
use crate::digest;
use crate::json_boundary;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[cfg(test)]
pub use json_patch::apply_patch;

#[derive(Default)]
pub(crate) struct SemanticCache {
    coverage_digests: DigestCache,
    plugin_policy_failures: BTreeMap<String, Vec<Failure>>,
}

pub fn semantic_failures(
    bundle: &Value,
    root: &Path,
    validator_digests: &BTreeMap<String, String>,
) -> Vec<Failure> {
    let mut cache = SemanticCache::default();
    semantic_failures_with_cache(bundle, root, validator_digests, &mut cache)
}

pub(crate) fn semantic_failures_with_cache(
    bundle: &Value,
    root: &Path,
    validator_digests: &BTreeMap<String, String>,
    cache: &mut SemanticCache,
) -> Vec<Failure> {
    let cm = &bundle["completion_manifest"];
    let lr = &bundle["lane_registry"];
    let ready = &bundle["ready_for_merge"];
    let ready_receipts = bundle
        .get("ready_for_merge_receipts")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let backlog = &bundle["verification_backlog"];
    let mut out = Vec::new();
    claim::identity::check_claim_ids(cm, &mut out);
    let validation_clock = lane::policy::validation_clock(bundle, &mut out);
    let claims = object_by_id(cm.get("claims"));
    let rows = object_by_id(backlog.get("rows"));

    for cid in json_boundary::string_array(cm, "required_claim_ids") {
        match claims.get(&cid) {
            None => out.push(Failure::new(
                "required-claim-closure",
                "required_claim_missing",
                cid,
            )),
            Some(claim) if str_field(claim, "claim_scope") != "required" => out.push(Failure::new(
                "claim-scope-closure",
                "required_claim_not_required_scope",
                cid,
            )),
            _ => {}
        }
    }

    ready::receipt::check_validator_receipt(bundle, ready, validator_digests, &mut out);
    ready::receipt::check_ready_receipt_set(bundle, &ready_receipts, &mut out);
    ready::join::check_ready_join(cm, lr, ready, &mut out);
    for claim in claims.values() {
        claim::proof::check_claim_with_cache(
            claim,
            &rows,
            cm,
            ready,
            root,
            &mut out,
            &mut cache.coverage_digests,
        );
    }
    bundle_hash_checks(bundle, &mut out);
    goal_checks(cm, &mut out);
    backlog_policy::check_backlog(backlog, &claims, &mut out);
    lane::policy::check_lanes(
        bundle,
        lr,
        ready,
        &ready_receipts,
        root,
        validation_clock,
        &mut out,
    );
    plugin_policy::check_plugin_with_cache(
        bundle,
        root,
        &mut out,
        &mut cache.plugin_policy_failures,
    );
    automation_tick::check(&bundle["automation_tick_receipt"], &mut out);
    amendment_checks(&bundle["amendments"], &mut out);
    ready::join::check_ready_integrity(lr, ready, &mut out);
    coverage::ready::join::check(root, ready, &mut out);
    authority_reconciliation::check(bundle, root, &mut out);
    out
}

pub(crate) fn object_by_id(value: Option<&Value>) -> BTreeMap<String, &Value> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|row| (str_field(row, "id"), row))
        .collect()
}

pub(crate) fn evidence(claim: &Value) -> Vec<&Value> {
    claim
        .get("evidence")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .collect()
}

pub(crate) fn str_field(value: &Value, key: &str) -> String {
    json_boundary::string(value, key).unwrap_or_default()
}

pub(crate) fn bool_field(value: &Value, key: &str) -> bool {
    json_boundary::bool_value(value, key).unwrap_or(false)
}

pub(crate) fn array_strings(value: &Value, key: &str) -> Vec<String> {
    json_boundary::string_array(value, key)
}

pub(crate) fn good_status(status: &str) -> bool {
    crate::audit::contract::GOOD_STATUSES.contains(&status)
}

pub(crate) fn path_contains(parent: &str, child: &str) -> bool {
    lane::paths::contains(parent, child).unwrap_or(false)
}

pub(crate) fn canonical_digest(value: &Value) -> Result<String, String> {
    Ok(digest::canonical_json(value))
}

pub(crate) fn intersects<T: Ord + AsRef<str>>(values: &BTreeSet<T>, words: &[&str]) -> bool {
    words
        .iter()
        .any(|word| values.iter().any(|value| value.as_ref() == *word))
}

fn bundle_hash_checks(bundle: &Value, out: &mut Vec<Failure>) {
    let manifest_hash = bundle
        .pointer("/completion_manifest/contract_bundle_hash")
        .and_then(Value::as_str)
        .unwrap_or("");
    let ready_hash = bundle
        .pointer("/ready_for_merge/contract_bundle_digest")
        .and_then(Value::as_str)
        .unwrap_or("");
    if manifest_hash == digest::ZERO || ready_hash == digest::ZERO || manifest_hash != ready_hash {
        out.push(Failure::new(
            "contract-bundle-hash",
            "contract_bundle_hash_mismatch",
            "ready receipt",
        ));
    }
}

fn goal_checks(cm: &Value, out: &mut Vec<Failure>) {
    let binding = &cm["goal_binding"];
    if str_field(binding, "status") == "unavailable"
        && binding
            .pointer("/discovery_probe_receipt/verdict")
            .and_then(Value::as_str)
            != Some("unavailable")
    {
        out.push(Failure::new(
            "goal-tool-discovery-probe",
            "goal_tools_unavailable_without_probe",
            "probe verdict mismatch",
        ));
    }
    if let Some(receipt) = binding.get("get_goal_receipt")
        && (str_field(receipt, "workspace") != str_field(cm, "root")
            || str_field(receipt, "observed_goal_id") != str_field(binding, "goal_id")
            || str_field(receipt, "observed_objective") != str_field(binding, "objective")
            || str_field(receipt, "observed_contract_path") != str_field(binding, "contract_path")
            || str_field(receipt, "observed_status") == "blocked")
    {
        out.push(Failure::new(
            "goal-binding-receipt-match",
            "goal_binding_receipt_stale_or_mismatched",
            "receipt mismatch",
        ));
    }
}

fn amendment_checks(amendments: &Value, out: &mut Vec<Failure>) {
    for amend in amendments.as_array().into_iter().flatten() {
        if str_field(amend, "change_class") == "clarifies"
            && amend
                .get("removed_or_weakened_claim_ids")
                .and_then(Value::as_array)
                .is_some_and(|ids| !ids.is_empty())
        {
            out.push(Failure::new(
                "amendment-monotonicity",
                "weakening_mislabeled_as_clarification",
                str_field(amend, "amendment_id"),
            ));
        } else if str_field(amend, "change_class") == "weakens"
            && amend
                .pointer("/approval/artifact/path")
                .and_then(Value::as_str)
                .unwrap_or("")
                .contains("missing")
        {
            out.push(Failure::new(
                "amendment-monotonicity",
                "weakening_without_explicit_approval",
                str_field(amend, "amendment_id"),
            ));
        }
    }
}
