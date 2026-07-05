use super::super::graph;
use super::super::surfaces::surface_by_id;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) const NODE_TIMING_REL: &str =
    "validation_artifacts/observability/live-loop-node-timing.json";

#[derive(Clone, Debug)]
pub(crate) struct NodeTiming {
    pub(crate) baseline_duration_ms: u64,
    pub(crate) verified_local_duration_ms: u64,
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
                || text(row, "timing_status")? != "pass"
                || text(row, "cache_honesty")? != "pass"
            {
                return None;
            }
            let baseline_duration_ms = positive(row, "baseline_duration_ms")?;
            let verified_local_duration_ms = positive(row, "verified_local_duration_ms")?;
            Some((
                node_id.to_string(),
                NodeTiming {
                    baseline_duration_ms,
                    verified_local_duration_ms,
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
        std::fs::remove_dir_all(root).expect("cleanup node timing");
    }
}
