use super::super::graph;
use super::super::surfaces::surface_by_id;
use super::command_failure::CommandFailureSummary;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) const NODE_TIMING_REL: &str =
    "validation_artifacts/observability/live-loop-node-timing.json";

#[derive(Clone, Debug)]
pub(crate) struct NodeTiming {
    pub(crate) baseline_duration_ms: u64,
    pub(crate) verified_local_duration_ms: u64,
    pub(crate) timing_status: String,
    pub(crate) failure_class: String,
    pub(crate) baseline_exit_code: Option<i32>,
    pub(crate) baseline_launch_error: bool,
    pub(crate) baseline_failure: CommandFailureSummary,
    pub(crate) affected_set_status: String,
    pub(crate) timing_source: String,
}

pub(crate) fn read_current(
    root: &Path,
    candidate_digest: &str,
    tier: &str,
    cache_mode: &str,
    changed_files_digest: &str,
    audit_context_digest: &str,
) -> BTreeMap<String, NodeTiming> {
    let Ok(value) = crate::json_boundary::read_json(&root.join(NODE_TIMING_REL)) else {
        return BTreeMap::new();
    };
    node_rows(&value)
        .into_iter()
        .filter_map(|row| {
            let node_id = text(row, "node_id")?;
            let surface = surface_by_id(node_id)?;
            let expected_input = graph::surface_input_digest(
                surface,
                candidate_digest,
                changed_files_digest,
                audit_context_digest,
            );
            if text(row, "candidate_digest")? != candidate_digest
                || text(row, "tier")? != tier
                || text(row, "cache_mode")? != cache_mode
                || text(row, "input_digest")? != expected_input
                || text(row, "canonical_full_command")? != surface.canonical_full_command
                || text(row, "cache_honesty")? != "pass"
            {
                return None;
            }
            let baseline_duration_ms = positive(row, "baseline_duration_ms")?;
            let verified_local_duration_ms = positive(row, "verified_local_duration_ms")?;
            let baseline_exit_code = row
                .get("baseline_exit_code")
                .and_then(serde_json::Value::as_i64)
                .and_then(|value| i32::try_from(value).ok());
            Some((
                node_id.to_string(),
                NodeTiming {
                    baseline_duration_ms,
                    verified_local_duration_ms,
                    timing_status: text(row, "timing_status").unwrap_or("fail").to_string(),
                    failure_class: text(row, "failure_class")
                        .unwrap_or("live_loop_node_measurement_failed")
                        .to_string(),
                    baseline_exit_code,
                    baseline_launch_error: row
                        .get("baseline_launch_error")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false),
                    baseline_failure: CommandFailureSummary::from_value(
                        row.get("baseline_failure"),
                    ),
                    affected_set_status: text(row, "affected_set_status")
                        .unwrap_or("unknown")
                        .to_string(),
                    timing_source: NODE_TIMING_REL.to_string(),
                },
            ))
        })
        .collect()
}

pub(super) fn node_rows(value: &Value) -> Vec<&Value> {
    value
        .get("nodes")
        .and_then(Value::as_array)
        .map(|rows| rows.iter().collect())
        .unwrap_or_default()
}

pub(super) fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

pub(super) fn positive(value: &Value, key: &str) -> Option<u64> {
    let number = value.get(key).and_then(Value::as_u64)?;
    (number > 0).then_some(number)
}

#[cfg(test)]
mod tests {
    use super::{NODE_TIMING_REL, read_current};
    use serde_json::json;

    #[test]
    fn node_timing_reader_accepts_only_current_candidate_and_input() {
        let root =
            crate::self_tests::boundaries::workspace_fixtures::temp_root("live-loop-node-timing");
        let candidate = "sha256:current";
        let changed = crate::digest::bytes(b"");
        let context =
            crate::digest::bytes(format!("{candidate}:hot:verified-local:{changed}").as_bytes());
        let input = super::graph::surface_input_digest(
            super::surface_by_id("fmt_check").expect("fmt surface"),
            candidate,
            &changed,
            &context,
        );
        crate::json_boundary::write_json(
            &root.join(NODE_TIMING_REL),
            &json!({
                "nodes": [
                    {
                        "node_id": "fmt_check",
                        "candidate_digest": candidate,
                        "tier": "hot",
                        "cache_mode": "verified-local",
                        "input_digest": input,
                        "canonical_full_command": "cargo fmt --all --check",
                        "timing_status": "pass",
                        "cache_honesty": "pass",
                        "failure_class": "none",
                        "baseline_exit_code": 0,
                        "baseline_launch_error": false,
                        "baseline_duration_ms": 1000,
                        "verified_local_duration_ms": 2,
                        "affected_set_status": "clean_worktree_no_affected_files"
                    },
                    {
                        "node_id": "build_check",
                        "candidate_digest": "sha256:stale",
                        "tier": "hot",
                        "cache_mode": "verified-local",
                        "input_digest": "sha256:wrong",
                        "canonical_full_command": "cargo build --offline --bin ultragoal --quiet",
                        "timing_status": "pass",
                        "cache_honesty": "pass",
                        "failure_class": "none",
                        "baseline_duration_ms": 1000,
                        "verified_local_duration_ms": 2
                    }
                ]
            }),
        )
        .expect("timing artifact");

        let timings = read_current(
            &root,
            candidate,
            "hot",
            "verified-local",
            &changed,
            &context,
        );
        assert_eq!(timings.len(), 1);
        assert_eq!(
            timings["fmt_check"].affected_set_status,
            "clean_worktree_no_affected_files"
        );
        assert_eq!(timings["fmt_check"].timing_status, "pass");
        assert_eq!(timings["fmt_check"].failure_class, "none");
        assert_eq!(timings["fmt_check"].baseline_exit_code, Some(0));
        std::fs::remove_dir_all(root).expect("cleanup node timing");
    }

    #[test]
    fn node_timing_reader_preserves_current_failed_measurement_rows() {
        let root =
            crate::self_tests::boundaries::workspace_fixtures::temp_root("live-loop-failed-timing");
        let candidate = "sha256:current";
        let changed = crate::digest::bytes(b"");
        let context =
            crate::digest::bytes(format!("{candidate}:hot:verified-local:{changed}").as_bytes());
        let input = super::graph::surface_input_digest(
            super::surface_by_id("fmt_check").expect("fmt surface"),
            candidate,
            &changed,
            &context,
        );
        crate::json_boundary::write_json(
            &root.join(NODE_TIMING_REL),
            &json!({
                "nodes": [{
                    "node_id": "fmt_check",
                    "candidate_digest": candidate,
                    "tier": "hot",
                    "cache_mode": "verified-local",
                    "input_digest": input,
                    "canonical_full_command": "cargo fmt --all --check",
                    "timing_status": "fail",
                    "failure_class": "canonical_full_command_failed",
                    "baseline_exit_code": 101,
                    "baseline_launch_error": false,
                    "cache_honesty": "pass",
                    "baseline_duration_ms": 1000,
                    "verified_local_duration_ms": 2,
                    "affected_set_status": "clean_worktree_no_affected_files"
                }]
            }),
        )
        .expect("timing artifact");

        let timings = read_current(
            &root,
            candidate,
            "hot",
            "verified-local",
            &changed,
            &context,
        );
        assert_eq!(timings.len(), 1);
        assert_eq!(timings["fmt_check"].timing_status, "fail");
        assert_eq!(
            timings["fmt_check"].failure_class,
            "canonical_full_command_failed"
        );
        assert_eq!(timings["fmt_check"].baseline_exit_code, Some(101));
        std::fs::remove_dir_all(root).expect("cleanup failed timing");
    }
}
