mod claims;
mod freeze;
mod graph;
mod lease;

use crate::audit::contract::Failure;
use serde_json::Value;
use std::path::{Component, Path, PathBuf};

pub(super) fn safe_repo_path(root: &Path, raw: &str) -> Option<PathBuf> {
    let path = Path::new(raw);
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::RootDir))
    {
        return None;
    }
    let root = root.canonicalize().ok()?;
    let candidate = root.join(path).canonicalize().ok()?;
    candidate.starts_with(&root).then_some(candidate)
}

/// Reconciles root-owned successor projections as one authority boundary.
/// Older fixture bundles intentionally use the v1 lane shape.
pub(crate) fn check(bundle: &Value, root: &Path, out: &mut Vec<Failure>) {
    let registry = &bundle["lane_registry"];
    if registry.get("schema").and_then(Value::as_str) != Some("harness-ultragoal.lane-registry.v2")
    {
        return;
    }
    let Some(current) = load_current(root, "LANE_REGISTRY.json", out) else {
        return;
    };
    if current.get("schema") != registry.get("schema") {
        out.push(Failure::new(
            "authority-reconciliation",
            "lane_registry_bundle_mismatch",
            "LANE_REGISTRY.json",
        ));
        return;
    }
    source_context::check(&current, root, out);
    graph::check(&current, root, out);
    claims::check(&current, root, out);
    freeze::check(&current, root, out);
    lease::check(&current, root, out);
}

mod source_context {
    use super::*;
    use std::collections::BTreeSet;

    pub(super) fn check(registry: &Value, root: &Path, out: &mut Vec<Failure>) {
        let Some(refs) = registry
            .pointer("/source_context/refs")
            .and_then(Value::as_object)
        else {
            out.push(Failure::new(
                "authority-source",
                "source_context_refs_missing",
                "source_context.refs",
            ));
            return;
        };
        let required = [
            "board",
            "graph",
            "tools",
            "claims",
            "goal",
            "amendments",
            "handoff",
            "plan",
        ];
        let required_set = required.into_iter().collect::<BTreeSet<_>>();
        let actual_set = refs.keys().map(String::as_str).collect::<BTreeSet<_>>();
        if actual_set != required_set {
            out.push(Failure::new(
                "authority-source",
                "source_context_ref_keys_mismatch",
                "source_context.refs",
            ));
        }
        for key in required {
            let Some(row) = refs.get(key) else {
                continue;
            };
            let Some(path) = row.get("path").and_then(Value::as_str) else {
                out.push(Failure::new(
                    "authority-source",
                    "source_context_ref_path_missing",
                    key,
                ));
                continue;
            };
            let Some(path) = super::safe_repo_path(root, path) else {
                out.push(Failure::new(
                    "authority-source",
                    "source_context_ref_path_unsafe",
                    key,
                ));
                continue;
            };
            let digest = crate::digest::file(&path).unwrap_or_default();
            if row.get("validity").and_then(Value::as_str) != Some("current_exact")
                || row.get("invalidation").and_then(Value::as_str)
                    != Some("invalidate_on_any_byte_change")
                || row.get("digest").and_then(Value::as_str) != Some(digest.as_str())
            {
                out.push(Failure::new(
                    "authority-source",
                    "source_context_ref_digest_mismatch",
                    key,
                ));
            }
        }
    }
}

fn load_current(root: &Path, relative: &str, out: &mut Vec<Failure>) -> Option<Value> {
    let path = root.join(relative);
    match crate::json_boundary::read_json(&path) {
        Ok(value) => Some(value),
        Err(error) => {
            out.push(Failure::new(
                "authority-reconciliation",
                "current_authority_unavailable",
                format!("{relative}: {error}"),
            ));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::check;
    use serde_json::Value;
    use std::fs;
    use std::path::Path;

    #[test]
    fn current_root_candidate_reconciles() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root");
        let registry: Value = serde_json::from_str(
            &fs::read_to_string(root.join("LANE_REGISTRY.json")).expect("current registry"),
        )
        .expect("registry JSON");
        let bundle = serde_json::json!({"lane_registry": registry});
        let mut failures = Vec::new();
        check(&bundle, root, &mut failures);
        assert!(
            failures.is_empty(),
            "current root reconciliation failures: {failures:?}"
        );
    }
}
