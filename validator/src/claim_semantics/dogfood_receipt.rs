use crate::audit::contract::Failure;
use crate::claim_semantics::{evidence, good_status, str_field};
use crate::{json_boundary, schema_catalog};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

pub fn check(claim: &Value, root: &Path, out: &mut Vec<Failure>) {
    if !dogfood_claim_included(claim) {
        return;
    }
    let receipts = evidence(claim)
        .into_iter()
        .filter(|ev| str_field(ev, "kind") == "dogfood_receipt")
        .collect::<Vec<_>>();
    if receipts.is_empty() {
        push(out, "dogfood_receipt_required", claim, "missing");
        return;
    }
    for ev in receipts {
        if receipt_invalid(claim, ev, root) {
            push(out, "dogfood_receipt_invalid", claim, &str_field(ev, "id"));
        }
    }
}

fn dogfood_claim_included(claim: &Value) -> bool {
    if str_field(claim, "claim_ceiling_effect") != "included"
        && !good_status(&str_field(claim, "status"))
    {
        return false;
    }
    let text = crate::claim_semantics::claim::proof::claim_text(claim);
    let tokens = crate::claim::text::tokens(&text);
    crate::claim::language::dogfood_claim(&text, &tokens)
}

fn receipt_invalid(claim: &Value, ev: &Value, root: &Path) -> bool {
    if str_field(ev, "surface") != "root_integration" {
        return true;
    }
    if crate::package::artifact::refs::validate_object(root, ev, "dogfood receipt evidence")
        .is_err()
    {
        return true;
    }
    let Ok(receipt) = json_boundary::read_json(&root.join(str_field(ev, "path"))) else {
        return true;
    };
    let store = schema_catalog::load(root);
    if !schema_catalog::schema_errors(&store, "dogfood-receipt.schema.json", &receipt).is_empty() {
        return true;
    }
    str_field(&receipt, "claim_id") != str_field(claim, "id") || dogfood_semantics_invalid(&receipt)
}

pub(crate) fn dogfood_semantics_invalid(receipt: &Value) -> bool {
    let lanes = receipt
        .get("lanes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    lanes.len() < 2
        || !unique_lane_workspaces(&lanes)
        || !lanes.iter().all(clean_macro_lane)
        || !lanes.iter().any(lane_owed_repair_closed)
        || receipt
            .pointer("/root_owed_follow_up/status")
            .and_then(Value::as_str)
            != Some("closed")
        || receipt
            .pointer("/root_owed_follow_up/final_manifest_clean")
            .and_then(Value::as_bool)
            != Some(true)
        || receipt
            .pointer("/external_review/status")
            .and_then(Value::as_str)
            != Some("pass")
}

fn unique_lane_workspaces(lanes: &[Value]) -> bool {
    let mut ids = BTreeSet::new();
    let mut workspaces = BTreeSet::new();
    lanes.iter().all(|lane| {
        let id = str_field(lane, "lane_id");
        let workspace = str_field(lane, "workspace");
        !id.is_empty() && !workspace.is_empty() && ids.insert(id) && workspaces.insert(workspace)
    })
}

fn clean_macro_lane(lane: &Value) -> bool {
    lane.get("macro_lane").and_then(Value::as_bool) == Some(true)
        && lane.pointer("/cleanup/status").and_then(Value::as_str) == Some("cleaned")
        && lane
            .pointer("/cleanup/stale_worktree_present")
            .and_then(Value::as_bool)
            == Some(false)
}

fn lane_owed_repair_closed(lane: &Value) -> bool {
    lane.pointer("/lane_owed_repair/status")
        .and_then(Value::as_str)
        == Some("closed")
}

fn push(out: &mut Vec<Failure>, error: &str, claim: &Value, detail: &str) {
    out.push(Failure::new(
        "claim-status-ceiling",
        error,
        format!("{}:{detail}", str_field(claim, "id")),
    ));
}
