use serde_json::{Map, Value};
use std::path::Path;

pub(crate) const REQUIRED_LOOP_STAGES: &[&str] = &[
    "current_digest_first",
    "run_failing_command_once",
    "query_logs_metrics_traces_by_run_id",
    "explain_failure_before_manual_artifact_inspection",
    "repair_smallest_root_cause",
    "rerun_narrow_command",
    "compare_before_after_telemetry",
    "broad_audit_after_narrow_pass",
    "stale_telemetry_blocks_claims",
    "structured_feedback_feeds_next_pass",
];

pub(crate) const REQUIRED_SIGNAL_CLASSES: &[&str] = &[
    "latency",
    "traffic",
    "errors",
    "saturation",
    "freshness",
    "correlation",
    "redaction",
    "boundedness",
];

pub(super) fn check(root: &Path, value: &Value, out: &mut Vec<String>) {
    require_research_doctrine(value, out);
    super::research_inputs::check(value, out);
    require_named_rows(
        root,
        value,
        "operating_loop_inventory",
        "observability_loop",
        REQUIRED_LOOP_STAGES,
        out,
    );
    require_named_rows(
        root,
        value,
        "signal_inventory",
        "observability_signal",
        REQUIRED_SIGNAL_CLASSES,
        out,
    );
}

fn require_research_doctrine(value: &Value, out: &mut Vec<String>) {
    for required in [
        "agent_legibility_local_stack",
        "repair_validate_loop",
        "trace_whole_workflow",
        "freshness_prevents_false_conclusions",
        "four_golden_signals_for_cli",
        "wide_structured_events_with_bounded_context",
        "semantic_naming_across_telemetry",
    ] {
        if !value
            .pointer(&format!("/operating_loop/doctrine/{required}"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            out.push(format!(
                "observability_operating_doctrine_missing:{required}"
            ));
        }
    }
}

fn require_named_rows(
    root: &Path,
    value: &Value,
    inventory_key: &str,
    prefix: &str,
    required_rows: &[&str],
    out: &mut Vec<String>,
) {
    let Some(rows) = value.get(inventory_key).and_then(Value::as_object) else {
        out.push(format!("{prefix}_inventory_missing"));
        return;
    };
    for required in required_rows {
        let Some(row) = rows.get(*required) else {
            out.push(format!("{prefix}_inventory_row_missing:{required}"));
            continue;
        };
        let Some(object) = row.as_object() else {
            out.push(format!("{prefix}_inventory_row_not_object:{required}"));
            continue;
        };
        require_row(root, prefix, required, object, out);
    }
    for key in rows.keys() {
        if !required_rows.contains(&key.as_str()) {
            out.push(format!("{prefix}_inventory_unknown:{key}"));
        }
    }
}

fn require_row(
    root: &Path,
    prefix: &str,
    name: &str,
    row: &Map<String, Value>,
    out: &mut Vec<String>,
) {
    match row.get("fitting_status").and_then(Value::as_str) {
        Some("fitted") => require_fitted_evidence(root, prefix, name, row, out),
        Some(status @ ("partially_fitted" | "unfitted")) => {
            require_unfitted_metadata(prefix, name, row, out);
            out.push(format!("{prefix}_fitting_unfitted:{name}:{status}"));
        }
        Some(other) => out.push(format!("{prefix}_fitting_status_invalid:{name}:{other}")),
        None => out.push(format!("{prefix}_fitting_status_missing:{name}")),
    }
}

fn require_unfitted_metadata(
    prefix: &str,
    name: &str,
    row: &Map<String, Value>,
    out: &mut Vec<String>,
) {
    if !typed_row_accounting(row)
        || !super::row_contract::complete(row)
        || !non_empty_array(row, "missing_surfaces")
        || !non_empty_string(row, "claim_impact")
    {
        out.push(format!("{prefix}_fitting_missing_metadata:{name}"));
        return;
    }
    if !owner_tracking(row) {
        out.push(format!("{prefix}_fitting_missing_metadata:{name}"));
    }
}

fn typed_row_accounting(row: &Map<String, Value>) -> bool {
    non_empty_string(row, "operation")
        && non_empty_string(row, "validator_check_id")
        && row.get("fitted_surfaces").is_some_and(Value::is_array)
        && row.get("focused_tests").is_some_and(Value::is_array)
        && row.get("receipt_paths").is_some_and(Value::is_array)
        && row
            .get("live_query_proof_paths")
            .is_some_and(Value::is_array)
}

fn require_fitted_evidence(
    root: &Path,
    prefix: &str,
    name: &str,
    row: &Map<String, Value>,
    out: &mut Vec<String>,
) {
    let required = [
        non_empty_array(row, "fitted_surfaces"),
        empty_array(row, "missing_surfaces"),
        non_empty_string(row, "operation"),
        non_empty_string(row, "validator_check_id"),
        non_empty_array(row, "focused_tests"),
        non_empty_array(row, "receipt_paths"),
        non_empty_array(row, "live_query_proof_paths"),
        owner_tracking(row),
        super::row_contract::complete(row),
        non_empty_string(row, "claim_impact"),
    ];
    if required.into_iter().any(|ok| !ok) {
        out.push(format!("{prefix}_fitting_row_shape_only:{name}"));
        return;
    }
    super::proof::require_current_surface_receipts(root, name, row, out);
}

fn non_empty_string(row: &Map<String, Value>, key: &str) -> bool {
    row.get(key)
        .and_then(Value::as_str)
        .is_some_and(|value| !value.trim().is_empty())
}

fn non_empty_array(row: &Map<String, Value>, key: &str) -> bool {
    row.get(key)
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty())
}

fn empty_array(row: &Map<String, Value>, key: &str) -> bool {
    row.get(key)
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty)
}

fn owner_tracking(row: &Map<String, Value>) -> bool {
    non_empty_string(row, "current_owner_surface") && non_empty_string(row, "next_unfitted_surface")
}
