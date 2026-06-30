use serde_json::{Value, json};

pub(super) fn budget(mode: &str) -> Value {
    let profile = profile(mode).unwrap_or(PROFILES[2]);
    json!({
        "mode": mode,
        "profile": profile.profile,
        "target_ms": profile.target_ms,
        "hard_ceiling_ms": profile.hard_ceiling_ms,
        "claim_impact": "source_local_speed_budget_only_not_readiness"
    })
}

pub(crate) fn is_known_mode(mode: &str) -> bool {
    profile(mode).is_some()
}

#[derive(Clone, Copy)]
struct Profile {
    aliases: &'static [&'static str],
    profile: &'static str,
    target_ms: u64,
    hard_ceiling_ms: u64,
}

const PROFILES: &[Profile] = &[
    Profile {
        aliases: &["hot", "hot_edit_check", "hot-edit-check"],
        profile: "hot_edit_check",
        target_ms: 5_000,
        hard_ceiling_ms: 5_000,
    },
    Profile {
        aliases: &["focused", "focused_repair", "focused-repair"],
        profile: "focused_repair",
        target_ms: 15_000,
        hard_ceiling_ms: 15_000,
    },
    Profile {
        aliases: &[
            "standard",
            "source",
            "source-local",
            "source_local",
            "standard_source_local",
            "standard-source-local",
            "init",
            "fresh-init",
            "fresh_init",
            "retrofit",
        ],
        profile: "standard_source_local",
        target_ms: 30_000,
        hard_ceiling_ms: 60_000,
    },
    Profile {
        aliases: &["strict", "audit", "strict_local", "strict-local"],
        profile: "strict_local_source_audit",
        target_ms: 60_000,
        hard_ceiling_ms: 180_000,
    },
    Profile {
        aliases: &["strict_fixtures", "strict-fixtures"],
        profile: "strict_fixtures",
        target_ms: 60_000,
        hard_ceiling_ms: 180_000,
    },
    Profile {
        aliases: &["strict_coverage", "strict-coverage"],
        profile: "strict_coverage",
        target_ms: 60_000,
        hard_ceiling_ms: 180_000,
    },
    Profile {
        aliases: &["final", "strict_final", "strict-final"],
        profile: "strict_final_source_local",
        target_ms: 60_000,
        hard_ceiling_ms: 180_000,
    },
];

fn profile(mode: &str) -> Option<Profile> {
    PROFILES
        .iter()
        .copied()
        .find(|profile| profile.aliases.contains(&mode))
}

#[cfg(test)]
mod tests {
    use super::{budget, is_known_mode};

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
        assert!(is_known_mode("source"));
        assert!(is_known_mode("fresh-init"));
        assert!(!is_known_mode("unknown"));
    }
}
