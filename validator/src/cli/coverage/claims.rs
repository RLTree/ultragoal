pub(super) fn why_failed(status: &str, failures: &[String]) -> String {
    if status == "pass" {
        "none".to_string()
    } else {
        format!("coverage prove failed: {}", failures.join("; "))
    }
}

use super::CoverageMode;

pub(super) fn next_repair(status: &str, mode: CoverageMode) -> &'static str {
    match (status, mode) {
        ("pass", _) => "none",
        (_, CoverageMode::Strict) => {
            "query this run through observe logs/metrics/traces, repair exact coverage, stale coverage receipt digests, or uncovered records, then rerun coverage prove"
        }
        (_, CoverageMode::Routine) => {
            "repair routine coverage lineage, stale input digests, cache class, or boundary equivalence, then rerun coverage prove --mode routine"
        }
    }
}

pub(super) fn impact(status: &str, mode: CoverageMode) -> &'static str {
    match (status, mode) {
        ("pass", CoverageMode::Strict) => {
            "supports_complete_coverage_source_local_observability_only"
        }
        ("pass", CoverageMode::Routine) => "routine_repair_only_blocks_completion_adjacent_claims",
        _ => "coverage_prove_failed_blocks_readiness_release_completion_update_goal",
    }
}

pub(super) fn supported(status: &str, mode: CoverageMode) -> Vec<String> {
    match (status, mode) {
        ("pass", CoverageMode::Strict) => ["coverage_prove", "complete_coverage"]
            .into_iter()
            .map(ToString::to_string)
            .collect(),
        ("pass", CoverageMode::Routine) => ["routine_coverage_feedback"]
            .into_iter()
            .map(ToString::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

pub(super) fn blocked(mode: CoverageMode) -> Vec<String> {
    let mut claims = [
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
    .collect::<Vec<_>>();
    if mode == CoverageMode::Routine {
        claims.push("complete_coverage".to_string());
    }
    claims
}
