use super::{ObserveCommand, Path, TraceBackendRequest};
use serde_json::Value;

pub(crate) fn trace_query_for_test(root: &Path, command: &ObserveCommand) -> String {
    super::trace_query(root, command)
}

pub(crate) fn trace_operation_for_test(root: &Path, command: &ObserveCommand) -> Option<String> {
    super::trace_operation(root, command)
}

pub(crate) fn trace_backend_id_for_test(raw: &str) -> String {
    super::trace_backend_id(raw)
}

pub(crate) fn trace_attempt_timeout_seconds_for_test(timeout_ms: u64) -> String {
    super::trace_attempt_timeout_seconds(timeout_ms)
}

pub(crate) fn trace_backend_request_for_test(
    root: &Path,
    command: &ObserveCommand,
    tags: &str,
) -> String {
    match super::backend_request(root, command, tags) {
        TraceBackendRequest::TraceById(trace_id) => format!("trace_by_id:{trace_id}"),
        TraceBackendRequest::Search { tags, operation } => {
            format!("search:{}:{tags}", operation.unwrap_or_default())
        }
    }
}

pub(crate) fn trace_lookup_error_for_test(trace_id: &str, err: String) -> String {
    super::trace_lookup_error(trace_id, err)
}

pub(crate) fn trace_target_tags_for_test(event: &Value) -> Option<String> {
    super::TraceTargetTags::from_event(event).query_text()
}
