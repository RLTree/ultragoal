use serde_json::Value;
use std::collections::BTreeSet;

pub(super) const REQUIRED_STAGES: &[&str] = &[
    "trace_capture",
    "typed_feedback",
    "feedback_clustering",
    "eval_generation",
    "promptfoo_execution",
    "halo_ranking",
    "codex_handoff",
    "implementation_linkage",
    "narrow_validation",
    "before_after_telemetry",
    "promotion_to_law",
];

const FORBIDDEN_SUBSTITUTIONS: &[&str] = &[
    "raw_trace_as_loop_closure",
    "raw_feedback_as_loop_closure",
    "raw_model_output_as_loop_closure",
    "raw_promptfoo_output_as_loop_closure",
    "raw_halo_output_as_loop_closure",
    "reviewer_agreement_as_loop_closure",
    "checklist_prose_as_loop_closure",
    "hand_authored_receipt_as_loop_closure",
];

pub(super) fn registry_failures(root: &std::path::Path, registry: &Value) -> Vec<String> {
    let mut out = Vec::new();
    require_str(
        registry,
        "schema",
        "harness-ultragoal.improvement-loop-registry.v1",
        &mut out,
    );
    require_str(registry, "law_id", super::LAW_ID, &mut out);
    let forbidden = array_strings(registry.get("forbidden_substitutions"));
    for substitution in FORBIDDEN_SUBSTITUTIONS {
        if !forbidden.contains(substitution) {
            out.push(format!(
                "improvement_loop_forbidden_substitution_missing:{substitution}"
            ));
        }
    }
    let loops = registry
        .get("loops")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if loops.is_empty() {
        out.push("improvement_loop_registry_has_no_loops".to_string());
    }
    for item in loops {
        loop_failures(root, &item, &mut out);
    }
    out
}

pub(super) fn loop_ids(registry: &Value) -> Vec<String> {
    registry
        .get("loops")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("loop_id").and_then(Value::as_str))
        .map(str::to_string)
        .collect()
}

pub(super) fn complete_same_candidate(registry: &Value) -> bool {
    registry
        .get("loops")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|item| {
            item.get("loop_closure_status").and_then(Value::as_str)
                == Some("complete_same_candidate")
                && stage_set(item).is_superset(&required_stage_set())
        })
}

fn loop_failures(root: &std::path::Path, item: &Value, out: &mut Vec<String>) {
    if item
        .get("loop_id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .is_empty()
    {
        out.push("improvement_loop_missing_loop_id".to_string());
    }
    for field in [
        "trace_ids",
        "feedback_ids",
        "cluster_ids",
        "eval_ids",
        "promptfoo_suite_ids",
        "halo_ranking_ids",
        "codex_handoff_ids",
        "implementation_change_ids",
        "validation_receipt_ids",
        "before_after_telemetry_ids",
        "promotion_ids",
    ] {
        if item
            .get(field)
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
        {
            out.push(format!("improvement_loop_missing_binding:{field}"));
        }
    }
    let stages = stage_set(item);
    for stage in REQUIRED_STAGES {
        if !stages.contains(*stage) {
            out.push(format!("improvement_loop_missing_stage:{stage}"));
        }
    }
    if item
        .get("loop_closure_status")
        .and_then(Value::as_str)
        .unwrap_or("")
        != "complete_same_candidate"
    {
        out.push("improvement_loop_not_complete_same_candidate".to_string());
    }
    out.extend(super::evidence::stage_evidence_failures(root, item));
}

fn stage_set(item: &Value) -> BTreeSet<&str> {
    item.get("stage_statuses")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|stage| {
            let id = stage.get("stage_id").and_then(Value::as_str)?;
            let status = stage.get("status").and_then(Value::as_str)?;
            (status == "complete").then_some(id)
        })
        .collect()
}

fn required_stage_set() -> BTreeSet<&'static str> {
    REQUIRED_STAGES.iter().copied().collect()
}

pub(super) fn require_str(value: &Value, key: &str, expected: &str, out: &mut Vec<String>) {
    if value.get(key).and_then(Value::as_str) != Some(expected) {
        out.push(format!("improvement_loop_field_mismatch:{key}"));
    }
}

pub(super) fn array_strings(value: Option<&Value>) -> BTreeSet<&str> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}
