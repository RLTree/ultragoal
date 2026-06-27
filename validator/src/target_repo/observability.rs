use crate::target_repo::artifact_refs;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

const REQUIRED: &[&str] = &[
    "docs/observability.md",
    "validation_artifacts/observability/events.jsonl",
    "validation_artifacts/observability/agent-context.md",
];
pub fn check(
    repo: &Path,
    markers: &[String],
    required: bool,
    checks: &mut serde_json::Map<String, Value>,
) {
    let marker_set = markers.iter().cloned().collect::<BTreeSet<_>>();
    let missing = missing(repo);
    let root = observability_root(repo);
    let error = if missing.is_empty() {
        observability_error(repo)
    } else {
        None
    };
    let marker = marker_set.contains("observability");
    let (status, detail) = if required && (!missing.is_empty() || !marker || error.is_some()) {
        (
            "blocked",
            format!(
                "requested observability stack missing: {}",
                missing
                    .first()
                    .cloned()
                    .or(error)
                    .unwrap_or_else(|| "gate marker".to_string())
            ),
        )
    } else if root.is_some() {
        let ok = marker && error.is_none();
        (
            if ok { "pass" } else { "fail" },
            if ok {
                "agent observability event ledger and query surface complete".to_string()
            } else {
                error.unwrap_or_else(|| "observability marker missing".to_string())
            },
        )
    } else {
        (
            "not_applicable",
            "observability not requested for runnable repo".to_string(),
        )
    };
    checks.insert(
        "observability-stack".to_string(),
        crate::target_repo::row(repo, status, &detail, root.as_deref()),
    );
}

fn observability_root(repo: &Path) -> Option<String> {
    if REQUIRED
        .iter()
        .all(|rel| crate::target_repo::safe_fs::is_nonempty_file(repo, rel))
    {
        Some("validation_artifacts/observability".to_string())
    } else {
        None
    }
}

fn missing(repo: &Path) -> Vec<String> {
    REQUIRED
        .iter()
        .filter(|rel| !crate::target_repo::safe_fs::is_nonempty_file(repo, rel))
        .map(|rel| rel.to_string())
        .collect()
}

fn observability_error(repo: &Path) -> Option<String> {
    let text = match crate::target_repo::safe_fs::read_to_string(
        repo,
        "validation_artifacts/observability/events.jsonl",
    ) {
        Ok(text) => text,
        Err(err) => return Some(format!("events.jsonl unreadable: {err}")),
    };
    let mut found = false;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let row: Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(err) => return Some(format!("events.jsonl malformed row: {err}")),
        };
        for key in [
            "schema",
            "repo",
            "commit",
            "worktree",
            "actor_id",
            "event_kind",
            "target_id",
            "status",
            "observed_at",
            "artifact",
        ] {
            if row.get(key).is_none() {
                return Some("events.jsonl row lacks required fields".to_string());
            }
        }
        found = true;
        if let Some(error) = artifact_refs::artifact_ref_error(
            repo,
            &row["artifact"],
            "observability event artifact",
        ) {
            return Some(error);
        }
    }
    if !found {
        return Some("events.jsonl lacks a complete event row".to_string());
    }
    observe_query_surface_error(repo)
}

fn observe_query_surface_error(repo: &Path) -> Option<String> {
    for rel in ["scripts/observe", "scripts/observe.sh"] {
        if !crate::target_repo::safe_fs::is_regular_file(repo, rel) {
            continue;
        }
        return unsafe_query_script_error(repo, rel);
    }
    Some("observability query command missing: scripts/observe".to_string())
}

fn unsafe_query_script_error(repo: &Path, rel: &str) -> Option<String> {
    let text = match crate::target_repo::safe_fs::read_to_string(repo, rel) {
        Ok(text) => text,
        Err(err) => return Some(format!("observability query command unreadable: {err}")),
    };
    if text.trim().is_empty() {
        return Some(format!("observability query command empty: {rel}"));
    }
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if !allowed_query_line(line) {
            return Some(format!(
                "observability query command is not static/read-only: {rel}"
            ));
        }
    }
    None
}

fn allowed_query_line(line: &str) -> bool {
    matches!(
        line,
        "#!/usr/bin/env bash"
            | "set -euo pipefail"
            | "query=\"${1:---latest}\""
            | "case \"$query\" in"
            | "--latest|latest) ;;"
            | "*)"
            | "echo \"unsupported observability query: $query\" >&2"
            | "exit 64"
            | ";;"
            | "esac"
            | "events=\"validation_artifacts/observability/events.jsonl\""
            | "context=\"validation_artifacts/observability/agent-context.md\""
            | "test -s \"$events\""
            | "test -s \"$context\""
            | "events_digest=\"$(shasum -a 256 \"$events\" | awk '{print $1}')\""
            | "context_digest=\"$(shasum -a 256 \"$context\" | awk '{print $1}')\""
            | "printf '{'"
            | "printf '\"schema\":\"harness-ultragoal.observability-query.v1\",'"
            | "printf '\"status\":\"pass\",'"
            | "printf '\"query\":\"latest\",'"
            | "printf '\"events\":{\"path\":\"%s\",\"digest\":\"sha256:%s\"},' \"$events\" \"$events_digest\""
            | "printf '\"agent_context\":{\"path\":\"%s\",\"digest\":\"sha256:%s\"}' \"$context\" \"$context_digest\""
            | "printf '}\\n'"
    )
}
