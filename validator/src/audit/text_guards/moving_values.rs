use std::path::Path;

const MOVING_VALUE_PATHS: &[&str] = &[
    "README.md",
    "REPORT.md",
    "docs/install-and-visibility.md",
    "docs/review-target-and-archive.md",
    "docs/source-obligation-matrix.md",
    "docs/source-obligation-matrix.json",
    "skills/proof-gate/SKILL.md",
];

pub fn failures(root: &Path) -> Vec<String> {
    MOVING_VALUE_PATHS
        .iter()
        .flat_map(|rel| {
            std::fs::read_to_string(root.join(rel))
                .map(|text| text_failures(rel, &text))
                .unwrap_or_default()
        })
        .collect()
}

fn text_failures(rel: &str, text: &str) -> Vec<String> {
    text.lines()
        .enumerate()
        .filter_map(|(index, line)| line_failure(rel, index + 1, line))
        .collect()
}

fn line_failure(rel: &str, line_number: usize, line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    if lower.contains("validation_artifacts/")
        || lower.contains("historical")
        || lower.contains("example")
    {
        return None;
    }
    let has_digest = lower.contains("sha256:") && has_sha_tail(&lower);
    let has_count = ["checks", "red fixtures", "inventory", "generated artifacts"]
        .iter()
        .any(|term| lower.contains(term) && has_slash_count(&lower));
    let has_run_id = lower.contains("ultragoal-audit-20");
    if has_digest || has_count || has_run_id {
        Some(format!(
            "{rel}:{line_number}: moving_value_drift_in_stable_text"
        ))
    } else {
        None
    }
}

fn has_sha_tail(text: &str) -> bool {
    text.split("sha256:").skip(1).any(|tail| {
        tail.chars()
            .take(64)
            .filter(|c| c.is_ascii_hexdigit())
            .count()
            == 64
    })
}

fn has_slash_count(text: &str) -> bool {
    text.split_whitespace().any(|word| {
        let trimmed = word.trim_matches(|c: char| !c.is_ascii_digit() && c != '/');
        let Some((left, right)) = trimmed.split_once('/') else {
            return false;
        };
        !left.is_empty()
            && !right.is_empty()
            && left.chars().all(|c| c.is_ascii_digit())
            && right.chars().all(|c| c.is_ascii_digit())
    })
}
