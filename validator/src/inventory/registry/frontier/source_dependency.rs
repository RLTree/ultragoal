use serde_json::Value;

pub(super) fn satisfies(registry: &Value, lane: &str, dependency: &str) -> bool {
    if lane != "N12"
        || dependency != "N11"
        || !matches!(
            registry
                .pointer("/pre_adoption_source/frontier")
                .and_then(Value::as_str),
            Some(
                "N02_REOBSERVED_N12_INTEGRATED_N14_READY_SOURCE_FRONTIER"
                    | "N14_ACTIVE_N12_INTEGRATED_SOURCE_FRONTIER"
            )
        )
    {
        return false;
    }
    registry
        .get("lanes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|row| row.get("id").and_then(Value::as_str) == Some("N11"))
        .is_some_and(|row| {
            row.get("state").and_then(Value::as_str) == Some("blocked")
                && row.get("ceiling").and_then(Value::as_str) == Some("source_accepted")
                && row
                    .pointer("/outcome/source_acceptance")
                    .and_then(Value::as_str)
                    == Some("accepted")
                && row
                    .pointer("/outcome/execution_outcome")
                    .and_then(Value::as_str)
                    == Some("external_blocked")
                && row
                    .pointer("/outcome/claim_availability")
                    .and_then(Value::as_str)
                    == Some("withheld")
        })
}
