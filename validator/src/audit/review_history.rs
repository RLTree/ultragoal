use std::collections::BTreeMap;
use std::path::Path;

const REVIEW_RECORD: &str = "docs/review-loop-record.md";
const STALE_CURRENT_PHRASES: &[&str] = &[
    "## Round 35 Current Sign-Off",
    "current internal four-reviewer sign-off",
];

pub fn check(root: &Path, failures: &mut BTreeMap<String, Vec<String>>) {
    let Ok(body) = std::fs::read_to_string(root.join(REVIEW_RECORD)) else {
        push(failures, "review loop record missing");
        return;
    };
    for phrase in STALE_CURRENT_PHRASES {
        if body.contains(phrase) {
            push(
                failures,
                format!("stale current sign-off authority phrase: {phrase}"),
            );
        }
    }
}

fn push(failures: &mut BTreeMap<String, Vec<String>>, detail: impl Into<String>) {
    failures
        .entry("validator-execution-provenance".to_string())
        .or_default()
        .push(detail.into());
}
