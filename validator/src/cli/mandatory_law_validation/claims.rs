pub(super) fn why_failed(status: &str, failures: &[String]) -> String {
    if status == "pass" {
        return "none".to_string();
    }
    let shown = failures.iter().take(12).cloned().collect::<Vec<_>>();
    let suffix = if failures.len() > shown.len() {
        format!("; and {} more", failures.len() - shown.len())
    } else {
        String::new()
    };
    format!(
        "mandatory law validation failed: {}{suffix}",
        shown.join("; ")
    )
}

pub(super) fn next_repair(status: &str) -> &'static str {
    if status == "pass" {
        "none"
    } else {
        "query this run through observe logs/metrics/traces, repair the named mandatory-law row, fixture, dependency, or evidence digest, then rerun mandatory-law validation"
    }
}

pub(super) fn impact(status: &str) -> &'static str {
    if status == "pass" {
        "supports_mandatory_law_validation_source_local_observability_only"
    } else {
        "mandatory_law_validation_failed_blocks_readiness_release_completion_update_goal"
    }
}

pub(super) fn supported(status: &str) -> Vec<String> {
    if status == "pass" {
        vec!["mandatory_law_validation".to_string()]
    } else {
        Vec::new()
    }
}

pub(super) fn blocked() -> Vec<String> {
    [
        "completion",
        "readiness",
        "release",
        "reviewer_exposure",
        "app_registry_exposure",
        "final_packet_correctness",
        "update_goal_eligibility",
    ]
    .into_iter()
    .map(ToString::to_string)
    .collect()
}
