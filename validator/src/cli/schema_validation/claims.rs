pub(super) fn why_failed(status: &str, failures: &[String]) -> String {
    if status == "pass" {
        "none".to_string()
    } else {
        format!("schema validation failed: {}", failures.join("; "))
    }
}

pub(super) fn next_repair(status: &str) -> &'static str {
    if status == "pass" {
        "none"
    } else {
        "query this run through observe logs/metrics/traces, repair the named schema catalog, mapped artifact, or target file, then rerun schema validation"
    }
}

pub(super) fn impact(status: &str) -> &'static str {
    if status == "pass" {
        "supports_schema_validation_source_local_observability_only"
    } else {
        "schema_validation_failed_blocks_readiness_release_completion_update_goal"
    }
}

pub(super) fn supported(status: &str) -> Vec<String> {
    if status == "pass" {
        vec!["schema_validation".to_string()]
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
