use crate::audit::contract::Failure;
use crate::claim_semantics::str_field;
use serde_json::Value;
use std::collections::BTreeSet;

pub(crate) fn check_claim_ids(cm: &Value, out: &mut Vec<Failure>) {
    let mut seen = BTreeSet::new();
    for claim in cm
        .get("claims")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let id = str_field(claim, "id");
        if id.is_empty() {
            continue;
        }
        if !seen.insert(id.clone()) {
            out.push(Failure::new(
                "claim-scope-closure",
                "duplicate_completion_claim_id",
                id,
            ));
        }
    }
}
