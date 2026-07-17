use serde_json::Value;

pub(super) fn receipt_surface_failures(value: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let Some(graph) = value.get("evidence_graph") else {
        out.push("cli_control_plane_receipt_missing_evidence_graph".to_string());
        return out;
    };
    if graph.get("evaluation_mode").and_then(Value::as_str) != Some("production_dereferenced") {
        out.push("cli_control_plane_receipt_evidence_graph_not_production".to_string());
    }
    let candidate = value
        .get("candidate_digest")
        .and_then(Value::as_str)
        .unwrap_or("");
    if graph.get("candidate_digest").and_then(Value::as_str) != Some(candidate) {
        out.push("cli_control_plane_receipt_evidence_graph_candidate_mismatch".to_string());
    }
    let operation = value.get("operation").and_then(Value::as_str).unwrap_or("");
    if graph.get("operation").and_then(Value::as_str) != Some(operation) {
        out.push("cli_control_plane_receipt_evidence_graph_operation_mismatch".to_string());
    }
    if graph.get("items").and_then(Value::as_array).is_none() {
        out.push("cli_control_plane_receipt_evidence_graph_missing_items".to_string());
    }
    out
}

pub(super) fn same_candidate_pass_failures(
    value: &Value,
    expected_candidate: &str,
    expected_operation: &str,
) -> Vec<String> {
    let mut out = graph_binding_failures(value, expected_candidate, expected_operation);
    if !operation_failures(value).is_empty() {
        out.push("cli_control_plane_receipt_evidence_graph_has_operation_failures".to_string());
    }
    for item in items(value) {
        if !item_failures(item).is_empty() {
            out.push(format!(
                "cli_control_plane_receipt_evidence_graph_item_has_failures:{}",
                label(item)
            ));
        }
        if item.get("exists").and_then(Value::as_bool) != Some(true) {
            out.push(format!(
                "cli_control_plane_receipt_evidence_graph_item_missing:{}",
                label(item)
            ));
        }
        if item.get("status").and_then(Value::as_str) != Some("pass") {
            out.push(format!(
                "cli_control_plane_receipt_evidence_graph_item_not_pass:{}",
                label(item)
            ));
        }
        if item.get("same_candidate").and_then(Value::as_bool) != Some(true) {
            out.push(format!(
                "cli_control_plane_receipt_evidence_graph_item_not_same_candidate:{}",
                label(item)
            ));
        }
    }
    out
}

pub(super) fn same_candidate_fail_closed_failures(
    value: &Value,
    expected_candidate: &str,
    expected_operation: &str,
) -> Vec<String> {
    let out = graph_binding_failures(value, expected_candidate, expected_operation);
    if !out.is_empty() {
        return out;
    }
    let has_item_failure = items(value).iter().any(|item| {
        !item_failures(item).is_empty()
            || item.get("exists").and_then(Value::as_bool) == Some(false)
            || item.get("status").and_then(Value::as_str) != Some("pass")
            || item.get("same_candidate").and_then(Value::as_bool) == Some(false)
    });
    if operation_failures(value).is_empty() && !has_item_failure {
        return vec!["cli_control_plane_receipt_evidence_graph_not_fail_closed".to_string()];
    }
    Vec::new()
}

fn graph_binding_failures(
    value: &Value,
    expected_candidate: &str,
    expected_operation: &str,
) -> Vec<String> {
    let mut out = receipt_surface_failures(value);
    let Some(graph) = value.get("evidence_graph") else {
        return out;
    };
    if graph.get("candidate_digest").and_then(Value::as_str) != Some(expected_candidate) {
        out.push("cli_control_plane_receipt_evidence_graph_wrong_candidate".to_string());
    }
    if graph.get("operation").and_then(Value::as_str) != Some(expected_operation) {
        out.push("cli_control_plane_receipt_evidence_graph_wrong_operation".to_string());
    }
    for label in required_labels(expected_operation) {
        if !items(value).iter().any(|item| item_label_is(item, label)) {
            out.push(format!(
                "cli_control_plane_receipt_evidence_graph_missing_label:{label}"
            ));
        }
    }
    out
}

fn required_labels(operation: &str) -> &'static [&'static str] {
    const BASE: &[&str] = &[
        "source_audit",
        "red_fixture_report",
        "coverage",
        "cli_performance",
        "final_packet",
        "registry_exposure",
        "fit_repo",
        "product_fitness",
        "product_journey",
        "standards_gardener",
        "install_audit",
        "cache_audit",
        "rust_toolchain",
        "rust_fast",
        "rust_standard",
        "rust_release",
        "rust_clean_proof",
        "rust_watch",
        "rust_memory",
        "rust_dependency",
        "rust_coverage",
        "rust_workspace_topology",
        "gc_plan",
        "gc_dry_run",
        "gc_apply",
        "gc_verify",
    ];
    const UPDATE_GOAL: &[&str] = &[
        "source_audit",
        "red_fixture_report",
        "coverage",
        "cli_performance",
        "final_packet",
        "registry_exposure",
        "fit_repo",
        "product_fitness",
        "product_journey",
        "standards_gardener",
        "install_audit",
        "cache_audit",
        "rust_toolchain",
        "rust_fast",
        "rust_standard",
        "rust_release",
        "rust_clean_proof",
        "rust_watch",
        "rust_memory",
        "rust_dependency",
        "rust_coverage",
        "rust_workspace_topology",
        "gc_plan",
        "gc_dry_run",
        "gc_apply",
        "gc_verify",
        "transactional_finalization",
    ];
    match operation {
        "registry_probe" | "app_surface_probe" => &["registry_exposure"],
        "update_goal_eligibility" | "self_update_goal_eligibility" => UPDATE_GOAL,
        _ => BASE,
    }
}

fn items(value: &Value) -> Vec<&Value> {
    value
        .pointer("/evidence_graph/items")
        .and_then(Value::as_array)
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

fn operation_failures(value: &Value) -> Vec<&Value> {
    value
        .pointer("/evidence_graph/operation_failures")
        .and_then(Value::as_array)
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

fn item_failures(item: &Value) -> Vec<&Value> {
    item.get("failures")
        .and_then(Value::as_array)
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

fn item_label_is(item: &Value, expected: &str) -> bool {
    item.get("label").and_then(Value::as_str) == Some(expected)
}

fn label(item: &Value) -> &str {
    item.get("label")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
}
