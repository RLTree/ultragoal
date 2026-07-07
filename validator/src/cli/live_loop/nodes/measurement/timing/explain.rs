use super::verified_work::VerifiedLocalProof;

pub(crate) fn telemetry_explain_text<'a>(
    verified_local: &'a VerifiedLocalProof,
    field: &str,
) -> Option<&'a str> {
    verified_local
        .telemetry_reconciliation
        .value
        .get("explain_failure")
        .or_else(|| {
            verified_local
                .telemetry_reconciliation
                .value
                .get("cached_reconciliation")
                .and_then(|cached| cached.get("explain_failure"))
        })
        .and_then(|receipt| receipt.get("value"))
        .and_then(|value| value.get(field))
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
}
