pub(super) fn matches(pattern: &str, text: &str) -> bool {
    match pattern {
        "^sha256:[0-9a-f]{64}$" => sha(text, false),
        "^sha256:[a-fA-F0-9]{64}$" => sha(text, true),
        "^[a-fA-F0-9]{7,64}$" => hex_range(text, 7, 64),
        "^CLAIM-[0-9]{3,}$" => id_prefix(text, "CLAIM-", 3),
        "^LANE-[0-9]{3,}$" => id_prefix(text, "LANE-", 3),
        "^AMEND-[0-9]{3,}$" => id_prefix(text, "AMEND-", 3),
        "^BACKLOG-[0-9]{3,}$" => id_prefix(text, "BACKLOG-", 3),
        "^[a-z0-9_]+$" => snake_token(text),
        "^[a-z0-9][a-z0-9-]*$" => kebab_token(text),
        "^[a-z0-9][a-z0-9-]*[a-z0-9]$" => strict_kebab_token(text),
        "(^examples/generated/|READY_FOR_MERGE|VALIDATOR_RECEIPT)" => {
            text.starts_with("examples/generated/")
                || text.contains("READY_FOR_MERGE")
                || text.contains("VALIDATOR_RECEIPT")
        }
        "^examples/generated/" => text.starts_with("examples/generated/"),
        "^custom-agents/harness-[a-z-]+\\.toml$" => harness_agent_toml(text),
        "^validation_artifacts/review/[-A-Za-z0-9._/]+[.]json$" => {
            artifact_json_under("validation_artifacts/review/", text)
        }
        "^validation_artifacts/cli/[-A-Za-z0-9._/]+[.]json$" => {
            artifact_json_under("validation_artifacts/cli/", text)
        }
        "^validation_artifacts/coverage/[-A-Za-z0-9._/]+[.]json$" => {
            artifact_json_under("validation_artifacts/coverage/", text)
        }
        "^validation_artifacts/harness/[-A-Za-z0-9._/]+[.]json$" => {
            artifact_json_under("validation_artifacts/harness/", text)
        }
        "^validation_artifacts/ultragoal-audit/[-A-Za-z0-9._/]+[.]json$" => {
            artifact_json_under("validation_artifacts/ultragoal-audit/", text)
        }
        other if other.starts_with("(^|/)") && other.ends_with('$') => {
            file_suffix_pattern(other, text)
        }
        other => unsupported_pattern(other, text),
    }
}

fn sha(text: &str, mixed: bool) -> bool {
    text.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .chars()
                .all(|c| c.is_ascii_hexdigit() && (mixed || !c.is_ascii_uppercase()))
    })
}

fn hex_range(text: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&text.len()) && text.chars().all(|c| c.is_ascii_hexdigit())
}

fn id_prefix(text: &str, prefix: &str, min_digits: usize) -> bool {
    text.strip_prefix(prefix).is_some_and(|digits| {
        digits.len() >= min_digits && digits.chars().all(|c| c.is_ascii_digit())
    })
}

fn snake_token(text: &str) -> bool {
    text.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn kebab_token(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn strict_kebab_token(text: &str) -> bool {
    kebab_token(text) && text.chars().last().is_some_and(|c| c != '-')
}

fn harness_agent_toml(text: &str) -> bool {
    text.starts_with("custom-agents/harness-") && text.ends_with(".toml")
}

fn artifact_json_under(prefix: &str, text: &str) -> bool {
    let Some(tail) = text.strip_prefix(prefix) else {
        return false;
    };
    !tail.is_empty()
        && tail.ends_with(".json")
        && tail
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
        && tail
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '/'))
}

fn file_suffix_pattern(pattern: &str, text: &str) -> bool {
    let needle = pattern
        .trim_start_matches("(^|/)")
        .trim_end_matches('$')
        .replace("\\-", "-")
        .replace("\\.", ".");
    text == needle || text.ends_with(&format!("/{needle}"))
}

fn unsupported_pattern(pattern: &str, _text: &str) -> bool {
    let _ = pattern;
    false
}

#[cfg(test)]
mod tests {
    #[test]
    fn supported_patterns_accept_and_reject_typed_values() {
        assert!(super::matches(
            "^sha256:[0-9a-f]{64}$",
            &format!("sha256:{}", "a".repeat(64))
        ));
        assert!(!super::matches(
            "^sha256:[0-9a-f]{64}$",
            &format!("sha256:{}", "A".repeat(64))
        ));
        assert!(super::matches(
            "^sha256:[a-fA-F0-9]{64}$",
            &format!("sha256:{}", "A".repeat(64))
        ));
        assert!(super::matches("^[a-fA-F0-9]{7,64}$", "abcDEF0"));
        assert!(!super::matches("^[a-fA-F0-9]{7,64}$", "abc"));
        assert!(super::matches("^CLAIM-[0-9]{3,}$", "CLAIM-001"));
        assert!(super::matches("^LANE-[0-9]{3,}$", "LANE-999"));
        assert!(super::matches("^AMEND-[0-9]{3,}$", "AMEND-001"));
        assert!(super::matches("^BACKLOG-[0-9]{3,}$", "BACKLOG-001"));
        assert!(super::matches("^[a-z0-9_]+$", "snake_1"));
        assert!(!super::matches("^[a-z0-9_]+$", "Snake"));
        assert!(super::matches("^[a-z0-9][a-z0-9-]*$", "kebab-1"));
        assert!(!super::matches("^[a-z0-9][a-z0-9-]*$", "-bad"));
        assert!(super::matches(
            "^[a-z0-9][a-z0-9-]*[a-z0-9]$",
            "strict-kebab-1"
        ));
        assert!(!super::matches(
            "^[a-z0-9][a-z0-9-]*[a-z0-9]$",
            "strict-kebab-"
        ));
        assert!(super::matches(
            "(^examples/generated/|READY_FOR_MERGE|VALIDATOR_RECEIPT)",
            "x/READY_FOR_MERGE.json"
        ));
        assert!(super::matches(
            "^examples/generated/",
            "examples/generated/a.json"
        ));
        assert!(super::matches(
            "^custom-agents/harness-[a-z-]+\\.toml$",
            "custom-agents/harness-reviewer.toml"
        ));
        assert!(super::matches(
            "^validation_artifacts/review/[-A-Za-z0-9._/]+[.]json$",
            "validation_artifacts/review/final-packet.json"
        ));
        assert!(!super::matches(
            "^validation_artifacts/review/[-A-Za-z0-9._/]+[.]json$",
            "validation_artifacts/other/final-packet.json"
        ));
        assert!(super::matches(
            "^validation_artifacts/cli/[-A-Za-z0-9._/]+[.]json$",
            "validation_artifacts/cli/update-goal-eligibility.json"
        ));
        assert!(!super::matches(
            "^validation_artifacts/cli/[-A-Za-z0-9._/]+[.]json$",
            "validation_artifacts/cli/../escape.json"
        ));
        assert!(super::matches(
            "^validation_artifacts/coverage/[-A-Za-z0-9._/]+[.]json$",
            "validation_artifacts/coverage/coverage-receipt.json"
        ));
        assert!(super::matches(
            "^validation_artifacts/harness/[-A-Za-z0-9._/]+[.]json$",
            "validation_artifacts/harness/fit-repo-receipt.json"
        ));
        assert!(super::matches(
            "^validation_artifacts/ultragoal-audit/[-A-Za-z0-9._/]+[.]json$",
            "validation_artifacts/ultragoal-audit/validator-receipt.json"
        ));
        assert!(super::matches(
            "(^|/)review\\-round\\.json$",
            "nested/review-round.json"
        ));
        assert!(!super::matches("unsupported", "anything"));
    }
}
