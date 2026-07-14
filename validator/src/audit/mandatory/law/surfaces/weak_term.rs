use serde_json::Value;

const TERMS: &str = "partial backlog blocked future follow-up reviewer-only documentation-only prose-only row-shape-only claim-ceiling-only stale-source-backed";

pub(super) fn contains(value: &Value) -> bool {
    let text = [
        "claim_ceiling_guard",
        "enforcement_status",
        "validator_check_id",
        "standards_row_id",
        "source_obligation_id",
    ]
    .iter()
    .filter_map(|key| value.get(*key).and_then(Value::as_str))
    .collect::<Vec<_>>()
    .join(" ")
    .to_ascii_lowercase();
    TERMS.split_whitespace().any(|term| text.contains(term))
}
