use serde_json::{Value, json};

pub(super) fn budget(mode: &str) -> Value {
    let (profile, target_ms, hard_ceiling_ms) = match mode {
        "hot" | "hot_edit_check" => ("hot_edit_check", 5_000, 5_000),
        "focused" => ("focused_repair", 15_000, 15_000),
        "standard" | "source-local" | "standard_source_local" | "init" => {
            ("standard_source_local", 30_000, 60_000)
        }
        "strict" | "audit" | "strict_local" => ("strict_local_source_audit", 60_000, 180_000),
        "strict_fixtures" => ("strict_fixtures", 60_000, 180_000),
        "strict_coverage" => ("strict_coverage", 60_000, 180_000),
        "final" | "strict_final" => ("strict_final_source_local", 60_000, 180_000),
        _ => ("standard_source_local", 30_000, 60_000),
    };
    json!({
        "mode": mode,
        "profile": profile,
        "target_ms": target_ms,
        "hard_ceiling_ms": hard_ceiling_ms,
        "claim_impact": "source_local_speed_budget_only_not_readiness"
    })
}

#[cfg(test)]
mod tests {
    use super::budget;

    #[test]
    fn speed_budget_profiles_match_corrected_iteration_targets() {
        let cases = [
            ("hot_edit_check", "hot_edit_check", 5_000, 5_000),
            ("focused", "focused_repair", 15_000, 15_000),
            (
                "standard_source_local",
                "standard_source_local",
                30_000,
                60_000,
            ),
            ("strict_local", "strict_local_source_audit", 60_000, 180_000),
            ("strict_fixtures", "strict_fixtures", 60_000, 180_000),
            ("strict_coverage", "strict_coverage", 60_000, 180_000),
            ("strict_final", "strict_final_source_local", 60_000, 180_000),
        ];
        for (mode, profile, target, hard_ceiling) in cases {
            let value = budget(mode);
            assert_eq!(value["profile"], profile);
            assert_eq!(value["target_ms"], target);
            assert_eq!(value["hard_ceiling_ms"], hard_ceiling);
            assert_eq!(
                value["claim_impact"],
                "source_local_speed_budget_only_not_readiness"
            );
        }
    }
}
