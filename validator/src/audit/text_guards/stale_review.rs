use serde_json::Value;
use std::path::Path;

const CURRENT_LAW_PATHS: &[&str] = &[
    "templates/AGENT_STANDARDS.md",
    "templates/agent-standards/05-review-and-completion.md",
    "templates/LANE_EXECPLAN.md",
    "templates/PLANS.md",
    "docs/exec-plans/active/rust-validator-migration-ultragoal.md",
    "docs/exec-plans/active/standards-orchestration-product-cohesion-ultragoal.md",
    "docs/hypercritical-review-law.md",
    "docs/plugin-resource-map.md",
    "docs/review-target-and-archive.md",
    "REPORT.md",
    "skills/execplan-lane/SKILL.md",
];

const STALE_REVIEW_PHRASES: &[&str] = &[
    "gpt-5.4-mini",
    "xhigh",
    "provisional approval",
    "provisionally approve",
    "candidate-freeze",
    "candidate freeze",
    "true-approval",
    "true approval",
    "true sign-off",
    "frozen anchors",
    "six canonical personas",
    "six-persona",
    "six reviewers",
    "all six",
    "same six personas",
    "contract adversary",
    "orchestration adversary",
    "verification gatekeeper",
    "simplicity auditor",
    "security/trust-boundary adversary",
    "production experience gatekeeper",
];

pub fn failures(root: &Path) -> Vec<String> {
    CURRENT_LAW_PATHS
        .iter()
        .flat_map(|rel| file_failures(root, rel))
        .collect()
}

pub fn value_failures(value: &Value) -> Vec<String> {
    let mut failures = Vec::new();
    json_failures(value, "materialized-json", &mut failures);
    failures
}

fn file_failures(root: &Path, rel: &str) -> Vec<String> {
    let text = std::fs::read_to_string(root.join(rel)).unwrap_or_default();
    text.lines()
        .enumerate()
        .flat_map(|(index, line)| line_failures(rel, index + 1, line))
        .collect()
}

fn json_failures(value: &Value, path: &str, failures: &mut Vec<String>) {
    match value {
        Value::String(text) => {
            failures.extend(line_failures(path, 1, text));
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                json_failures(item, &format!("{path}/{index}"), failures);
            }
        }
        Value::Object(map) => {
            for (key, item) in map {
                json_failures(item, &format!("{path}/{key}"), failures);
            }
        }
        _ => {}
    }
}

fn line_failures(rel: &str, line_number: usize, line: &str) -> Vec<String> {
    let lower = line.to_ascii_lowercase();
    STALE_REVIEW_PHRASES
        .iter()
        .filter(|phrase| lower.contains(&phrase.to_ascii_lowercase()))
        .map(|phrase| {
            format!("{rel}:{line_number}: stale_review_cadence_in_current_surface: {phrase}")
        })
        .collect()
}
