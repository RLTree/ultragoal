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
                    | "N14_EXTERNAL_BLOCKED_N12_INTEGRATED_SOURCE_ACCEPTED"
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

#[cfg(test)]
mod tests {
    use super::satisfies;
    use serde_json::json;

    fn registry(frontier: &str, source_acceptance: &str) -> serde_json::Value {
        json!({
            "pre_adoption_source": {"frontier": frontier},
            "lanes": [{
                "id": "N11",
                "state": "blocked",
                "ceiling": "source_accepted",
                "outcome": {
                    "source_acceptance": source_acceptance,
                    "execution_outcome": "external_blocked",
                    "claim_availability": "withheld"
                }
            }]
        })
    }

    #[test]
    fn accepted_n11_source_remains_a_dependency_after_n14_external_block() {
        let registry = registry(
            "N14_EXTERNAL_BLOCKED_N12_INTEGRATED_SOURCE_ACCEPTED",
            "accepted",
        );
        assert!(satisfies(&registry, "N12", "N11"));
    }

    #[test]
    fn external_block_does_not_substitute_for_source_acceptance() {
        let registry = registry(
            "N14_EXTERNAL_BLOCKED_N12_INTEGRATED_SOURCE_ACCEPTED",
            "rejected",
        );
        assert!(!satisfies(&registry, "N12", "N11"));
        assert!(!satisfies(&registry, "N13", "N11"));
    }
}
