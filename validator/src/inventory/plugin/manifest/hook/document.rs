use super::plugin_manifest::bounded_text;
use super::plugin_manifest_hooks::{HookMap, Inspection, inspect_map};
use crate::context::ReadSession;
use serde::Deserialize;
use std::path::Path;

const MAX_HOOK_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HookDocument {
    #[serde(default)]
    description: Option<String>,
    hooks: HookMap,
}

fn invalid(paths: Vec<String>) -> Inspection {
    Inspection {
        valid: false,
        inactive: false,
        trust_required: false,
        paths,
    }
}

pub(super) fn inspect(reads: &ReadSession, root: &Path, path: &str) -> Inspection {
    if !path.ends_with(".json") || super::plugin_manifest_path::relative(path).is_none() {
        return invalid(Vec::new());
    }
    let paths = vec![path.to_owned()];
    if !super::plugin_manifest_path::companion_exists(reads, root, path) {
        return invalid(paths);
    }
    let Ok(bytes) = reads.read_bounded(&root.join(path), MAX_HOOK_BYTES) else {
        return invalid(paths);
    };
    if !super::plugin_manifest_json::unique_keys(&bytes) {
        return invalid(paths);
    }
    let Ok(document) = serde_json::from_slice::<HookDocument>(&bytes) else {
        return invalid(paths);
    };
    if !document.description.as_deref().is_none_or(bounded_text) {
        return invalid(paths);
    }
    let mut inspection = inspect_map(&document.hooks);
    inspection.paths = paths;
    inspection
}
