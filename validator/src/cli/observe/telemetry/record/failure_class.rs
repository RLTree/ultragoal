use crate::cli::observe::command::ObserveOperation;

pub(super) fn for_observe_failure(
    operation: ObserveOperation,
    failure: Option<&str>,
) -> &'static str {
    let Some(failure) = failure else {
        return "none";
    };
    if failure.contains("observability_metric_event_time_stale") {
        return "observability_metric_event_time_stale";
    }
    if failure.contains("observability_metric_missing_for_target")
        || (matches!(operation, ObserveOperation::MetricsQuery)
            && failure.contains("observability query returned no matching rows"))
    {
        return "observability_metric_missing_for_target";
    }
    if failure.contains("observability_metric_target_unavailable") {
        return "observability_metric_target_unavailable";
    }
    if failure.contains("victoriatraces trace lookup returned 404")
        || (matches!(operation, ObserveOperation::TracesQuery)
            && failure.contains("observability query returned no matching rows"))
    {
        return "observability_trace_tree_unavailable";
    }
    if matches!(operation, ObserveOperation::LogsQuery)
        && failure.contains("observability query returned no matching rows")
    {
        return "observability_log_record_unavailable";
    }
    if failure.contains("curl query failed") && failure.contains("timed out") {
        return "observability_live_backend_timeout";
    }
    if failure.contains("requested telemetry target unavailable") {
        return "observability_target_unavailable";
    }
    if candidate_freshness_failure(failure) {
        return "observability_candidate_mismatch";
    }
    if failure.contains("observed telemetry failure is opaque") {
        return "observability_opaque_failure_output";
    }
    "observability_product_closure_failure"
}

fn candidate_freshness_failure(failure: &str) -> bool {
    [
        "observed telemetry candidate digest",
        "observability_query_candidate_digest_mismatch",
        "observability_logs_candidate_mismatch",
        "observability_logs_candidate_missing",
        "observability_metric_candidate_mismatch",
        "observability_metric_candidate_missing",
        "observability_traces_candidate_mismatch",
        "observability_traces_candidate_missing",
    ]
    .iter()
    .any(|prefix| failure.contains(prefix))
}
