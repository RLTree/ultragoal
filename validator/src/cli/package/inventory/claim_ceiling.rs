pub(super) fn why_failed(status: &str, failures: &[String]) -> String {
    if status == "pass" {
        "none".to_string()
    } else {
        format!("package inventory check failed: {}", failures.join("; "))
    }
}

pub(super) fn next_repair(status: &str) -> &'static str {
    if status == "pass" {
        "none"
    } else {
        "repair the named missing, duplicate, stale, private, generic, or misclassified package resource, then rerun package inventory"
    }
}

pub(super) fn impact(status: &str) -> &'static str {
    if status == "pass" {
        "supports_package_inventory_source_local_observability_only"
    } else {
        "package_inventory_failed_blocks_package_readiness_release_completion_update_goal"
    }
}

pub(super) fn supported(status: &str) -> Vec<String> {
    if status == "pass" {
        vec!["package_inventory_source_local".to_string()]
    } else {
        Vec::new()
    }
}

pub(super) fn blocked() -> Vec<String> {
    [
        "completion",
        "package_readiness",
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
