pub(super) fn why_failed(status: &str, failures: &[String]) -> String {
    if status == "pass" {
        "none".to_string()
    } else {
        format!("foundational-trace check failed: {}", failures.join("; "))
    }
}

pub(super) fn next_repair(status: &str) -> &'static str {
    if status == "pass" {
        "none"
    } else {
        "query this run through observe logs/metrics/traces, repair the named foundational trace row, source-obligation row, standards row, validator check id, or fixture binding, then rerun foundational-trace check"
    }
}

pub(super) fn impact(status: &str) -> &'static str {
    if status == "pass" {
        "supports_foundational_trace_source_local_observability_only"
    } else {
        "foundational_trace_failed_blocks_readiness_release_completion_update_goal"
    }
}

pub(super) fn supported(status: &str) -> Vec<String> {
    if status == "pass" {
        vec!["foundational_trace_check".to_string()]
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
